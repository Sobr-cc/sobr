//! Dashboard + eco-budget (C19 — M15 + M25).

use chrono::{DateTime, Utc};
use sobria_core::{EstimationResult, Indicator};
use tracing::info;

use super::{lock, read_all_audit_entries};
use crate::{
    dashboard::{self, DashboardPeriod},
    dto::{BudgetStatusDto, DashboardSummaryDto, PersonalGoalDto},
    error::{IpcError, IpcResult},
    goals_store::{GoalIndicator, GoalPeriod, PersonalGoal},
    state::AppState,
};

/// Builds the dashboard summary for the given period.
pub fn get_dashboard_summary(period_str: &str, state: &AppState) -> IpcResult<DashboardSummaryDto> {
    let period = DashboardPeriod::parse(period_str).ok_or_else(|| {
        IpcError::invalid_request(format!(
            "period '{period_str}' inconnue (attendu: today | last_7_days | this_month | last_month | this_year)"
        ))
    })?;
    let entries = read_all_audit_entries(state)?;
    let summary = dashboard::aggregate(&entries, period, Utc::now());
    Ok((&summary).into())
}

/// Lists all personal goals (M25).
pub fn list_personal_goals(state: &AppState) -> IpcResult<Vec<PersonalGoalDto>> {
    let store = lock(&state.goals, "goals")?;
    let goals = store.list_all()?;
    Ok(goals.into_iter().map(Into::into).collect())
}

/// UPSERTs a personal goal.
pub fn set_personal_goal(dto: PersonalGoalDto, state: &AppState) -> IpcResult<()> {
    let goal = dto_to_goal(dto)?;
    goal.validate()?;
    let mut store = lock(&state.goals, "goals")?;
    store.upsert(&goal)?;
    info!(
        indicator = goal.indicator.as_str(),
        period = goal.period.as_str(),
        value_max = goal.value_max,
        "goals: set"
    );
    Ok(())
}

/// Deletes a goal (idempotent).
pub fn delete_personal_goal(indicator: &str, period: &str, state: &AppState) -> IpcResult<()> {
    let (i, p) = parse_goal_key(indicator, period)?;
    let mut store = lock(&state.goals, "goals")?;
    store.delete(i, p)?;
    Ok(())
}

/// Budget status for each defined goal.
pub fn get_budget_status(state: &AppState) -> IpcResult<Vec<BudgetStatusDto>> {
    let goals = lock(&state.goals, "goals")?.list_all()?;
    if goals.is_empty() {
        return Ok(vec![]);
    }
    let entries = read_all_audit_entries(state)?;
    let now = Utc::now();
    let targets: Vec<(Indicator, DateTime<Utc>, DateTime<Utc>)> = goals
        .iter()
        .map(|g| {
            let (start, end) = match g.period {
                GoalPeriod::Daily => DashboardPeriod::Today,
                GoalPeriod::Weekly => DashboardPeriod::Last7Days,
                GoalPeriod::Monthly => DashboardPeriod::ThisMonth,
            }
            .window(now);
            (g.indicator.indicator(), start, end)
        })
        .collect();
    let sums = sum_indicators_in_windows(&entries, &targets);
    let mut out = Vec::with_capacity(goals.len());
    for ((g, (_, start, end)), agg) in goals.into_iter().zip(targets).zip(sums) {
        let consumed_pct = if g.value_max > 0.0 {
            (agg / g.value_max) * 100.0
        } else {
            0.0
        };
        let status = if consumed_pct > 100.0 {
            "exceeded"
        } else if consumed_pct >= 70.0 {
            "warning"
        } else {
            "ok"
        };
        let remaining = g.value_max - agg;
        out.push(BudgetStatusDto {
            goal: g.into(),
            current_value: agg,
            period_start: start.to_rfc3339(),
            period_end: end.to_rfc3339(),
            consumed_pct,
            status: status.into(),
            remaining,
        });
    }
    Ok(out)
}

/// Parses a goal key (indicator first, then period).
fn parse_goal_key(indicator: &str, period: &str) -> IpcResult<(GoalIndicator, GoalPeriod)> {
    let i = GoalIndicator::parse(indicator)
        .ok_or_else(|| IpcError::invalid_request(format!("indicator '{indicator}' inconnu")))?;
    let p = GoalPeriod::parse(period)
        .ok_or_else(|| IpcError::invalid_request(format!("period '{period}' inconnue")))?;
    Ok((i, p))
}

fn dto_to_goal(dto: PersonalGoalDto) -> IpcResult<PersonalGoal> {
    let (indicator, period) = parse_goal_key(&dto.indicator, &dto.period)?;
    Ok(PersonalGoal {
        indicator,
        period,
        value_max: dto.value_max,
        unit: dto.unit,
    })
}

/// Sums each target's indicator P50 over the entries inside its `[start, end)`
/// window, in entry order; purged or unreadable entries are skipped.
fn sum_indicators_in_windows(
    entries: &[sobria_audit::AuditEntry],
    targets: &[(Indicator, DateTime<Utc>, DateTime<Utc>)],
) -> Vec<f64> {
    let mut sums = vec![0.0; targets.len()];
    for entry in entries {
        let in_window = |start: DateTime<Utc>, end: DateTime<Utc>| {
            entry.timestamp >= start && entry.timestamp < end
        };
        if entry.is_purged() || !targets.iter().any(|&(_, s, e)| in_window(s, e)) {
            continue;
        }
        let Ok(result) = serde_json::from_str::<EstimationResult>(&entry.estimation_result_json)
        else {
            continue;
        };
        for (sum, &(indicator, start, end)) in sums.iter_mut().zip(targets) {
            if in_window(start, end) {
                *sum += dashboard::indicator_p50(&result.indicators, indicator).unwrap_or(0.0);
            }
        }
    }
    sums
}
