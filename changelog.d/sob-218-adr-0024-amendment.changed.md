ADR-0024 amended: shared host, smaller cap, shared front end, portability (2026-10-31, SOB-218)

- **ADR-0024, Amendment 1** (decisions only; the facts of the host stay in the private
  host contract):
  - Sobr's starting cap is 10 GB of memory, 4 vCPU and 200 GB of disk (Thibault, SOB-213).
    The sizing of §8 does not fit: the lab becomes one small Talos node plus the router
    VM. The managed cloud does not open on this cap, and CI runners stay on GitHub's
    runners until the cap rises.
  - The EU front server is shared as a layer-4 relay on the host name, with no TLS
    termination and its own tunnel, keys and routes per project; tunnels are dialed out
    from the cluster. Backups live off the host: layers 1 and 2 and a copy in a third
    place.
  - The portability rule of SOB-211 becomes five numbered rules with their checks.
    ADR-0024 stays Proposed until the open facts are confirmed.
