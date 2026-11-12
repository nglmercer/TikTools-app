//! Depth-preserving context inheritance and conjunctive authority intersection.
use super::*;

impl ExecutionContext {
    pub(crate) fn depth(&self) -> u8 {
        self.depth
    }

    pub(crate) fn child(&self) -> Result<Self, &'static str> {
        let depth = self
            .depth
            .checked_add(1)
            .filter(|depth| *depth < DEPTH_LIMIT)
            .ok_or("automation emit depth limit reached")?;
        Ok(Self {
            authority: self.authority.clone(),
            depth,
            script_flow: self.script_flow,
            allow_delegation: self.allow_delegation,
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
            #[cfg(feature = "persistence")]
            competition: self.competition.clone(),
            #[cfg(feature = "persistence")]
            competition_winner: self.competition_winner.clone(),
        })
    }

    /// Owner is neutral; denied is absorbing; restricted subjects are combined
    /// conjunctively under one host service. Depth can never reset via nesting.
    pub(crate) fn intersect(&self, other: &Self) -> Result<Self, &'static str> {
        let authority = match (&self.authority, &other.authority) {
            (Authority::Denied, _) | (_, Authority::Denied) => Authority::Denied,
            (Authority::Owner, other) | (other, Authority::Owner) => other.clone(),
            (Authority::Restricted(a), Authority::Restricted(b)) => Authority::Restricted(
                a.intersect(b)
                    .map_err(|_| "execution subject intersection is unavailable")?,
            ),
            #[cfg(feature = "persistence")]
            (Authority::Restricted(a), Authority::Approved { subjects: b, .. })
            | (Authority::Approved { subjects: b, .. }, Authority::Restricted(a)) => {
                // Identity-only restrictions have no approval. Adding a valid
                // permit cannot erase the unapproved incoming restriction.
                Authority::Restricted(
                    a.intersect(b)
                        .map_err(|_| "execution subject intersection is unavailable")?,
                )
            }
            #[cfg(feature = "persistence")]
            (
                Authority::Approved {
                    subjects: a,
                    permits: ap,
                    owner_rules: ar,
                },
                Authority::Approved {
                    subjects: b,
                    permits: bp,
                    owner_rules: br,
                },
            ) => {
                if ap.len().saturating_add(bp.len()) > 16 || ar.len().saturating_add(br.len()) > 16
                {
                    return Err("execution approval intersection exceeds its bound");
                }
                Authority::Approved {
                    subjects: a
                        .intersect(b)
                        .map_err(|_| "execution subject intersection is unavailable")?,
                    permits: ap.iter().chain(bp).cloned().collect(),
                    owner_rules: ar.iter().chain(br).cloned().collect(),
                }
            }
        };
        let allow_delegation = self.allow_delegation && other.allow_delegation;
        if !allow_delegation && !matches!(authority, Authority::Owner | Authority::Denied) {
            return Err("unverified LIVE input cannot authorize delegated execution");
        }
        #[cfg(feature = "persistence")]
        let competition = match (&self.competition, &other.competition) {
            (Some(a), Some(b)) if a != b => return Err("competition execution bindings conflict"),
            (Some(fence), _) | (_, Some(fence)) => Some(fence.clone()),
            _ => None,
        };
        #[cfg(feature = "persistence")]
        let competition_winner = match (&self.competition_winner, &other.competition_winner) {
            (Some(a), Some(b)) if a != b => return Err("competition winner bindings conflict"),
            (Some(claim), _) | (_, Some(claim)) => Some(claim.clone()),
            _ => None,
        };
        Ok(Self {
            authority,
            depth: self.depth.max(other.depth),
            script_flow: self.script_flow || other.script_flow,
            allow_delegation,
            #[cfg(feature = "persistence")]
            execution: self.execution.clone(),
            #[cfg(feature = "persistence")]
            competition,
            #[cfg(feature = "persistence")]
            competition_winner,
        })
    }

    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) fn mark_script_flow(mut self) -> Self {
        self.script_flow = true;
        self
    }

    // Currently consumed only by the optional HTTP sink and scoped tests.
    #[cfg_attr(not(feature = "http"), allow(dead_code))]
    pub(crate) fn has_script_flow(&self) -> bool {
        self.script_flow
    }
}
