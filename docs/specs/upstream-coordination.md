# Upstream Coordination Specification

> Status: deferred

The MVP records origin and revision at import time but does not check whether an
origin has changed. It does not maintain baselines, compare base/local/upstream,
merge, hold, ignore, detach, or claim that an imported bundle is current.

If refresh becomes necessary, it requires a new scope decision and a design that
starts with a simple explicit replacement workflow rather than a background or
three-way update system.
