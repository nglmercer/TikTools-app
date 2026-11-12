//! Draft and proposal access serialized under the selected journal writer.
use super::*;

impl Journal {
    pub(in crate::remote) fn drafts(
        &self,
    ) -> Result<(String, Vec<crate::remote::profiles::drafts::Draft>), ClientError> {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| crate::remote::profiles::drafts::invalid())?;
        let data = crate::remote::profiles::drafts::read(
            &self.path.with_extension("drafts"),
            &self.view().binding,
        )?;
        Ok((data.revision, data.records.into_values().collect()))
    }

    pub(in crate::remote) fn change_draft(
        &self,
        expected: &str,
        id: &str,
        draft: Option<crate::remote::profiles::drafts::Draft>,
    ) -> Result<String, ClientError> {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| crate::remote::profiles::drafts::invalid())?;
        crate::remote::profiles::drafts::change(
            &self.path.with_extension("drafts"),
            &self.view().binding,
            expected,
            id,
            draft,
        )
    }

    pub(in crate::remote) fn proposals(
        &self,
    ) -> Result<
        (
            String,
            Vec<crate::remote::profiles::proposals::ProposalDraft>,
        ),
        ClientError,
    > {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| crate::remote::profiles::proposals::invalid())?;
        let data = crate::remote::profiles::proposals::read(
            &self.path.with_extension("proposals"),
            &self.view().binding,
        )?;
        Ok((data.revision, data.records.into_values().collect()))
    }

    pub(in crate::remote) fn change_proposal(
        &self,
        expected: &str,
        kind: tiktools_collaboration::AutomationFamily,
        id: &str,
        draft: Option<crate::remote::profiles::proposals::ProposalDraft>,
    ) -> Result<String, ClientError> {
        let _writing = self
            .writing
            .lock()
            .map_err(|_| crate::remote::profiles::proposals::invalid())?;
        crate::remote::profiles::proposals::change(
            &self.path.with_extension("proposals"),
            &self.view().binding,
            expected,
            kind,
            id,
            draft,
        )
    }
}
