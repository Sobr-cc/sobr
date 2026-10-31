//! Entry point of the `sobria-team-aggregator` binary.

fn main() -> anyhow::Result<()> {
    sobria_team_aggregator::cli::run()
}
