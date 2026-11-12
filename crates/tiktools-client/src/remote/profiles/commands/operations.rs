//! Command operation identities, capability requirements and journal version support.
use super::*;

impl Operation {
    pub fn method(self) -> &'static str {
        match self {
            Self::Disable => "workflows.disable",
            Self::Delete => "workflows.delete",
            Self::AdjustPoints => "points.adjust",
            Self::StartCompetition => "competitions.start",
            Self::ResumeCompetition => "competitions.resume",
            Self::PauseCompetition => "competitions.pause",
            Self::CancelCompetition => "competitions.cancel",
            Self::DisableEvent | Self::DisableAction => "automation.disable",
            Self::EnableEvent | Self::EnableAction => "automation.enable",
            Self::DeleteEvent => "automation.delete",
            Self::CreateEventDraft
            | Self::CreateActionDraft
            | Self::CreateEventActive
            | Self::CreateActionActive => "automation.create",
            Self::UpdateEventDraft
            | Self::UpdateActionDraft
            | Self::UpdateEventActive
            | Self::UpdateActionActive => "automation.update",
        }
    }
    pub fn feature(self) -> &'static str {
        match self {
            Self::Disable => "workflowDisable",
            Self::Delete => "workflowDelete",
            Self::AdjustPoints => "pointsAdjust",
            Self::StartCompetition | Self::ResumeCompetition => "competitionActivation",
            Self::PauseCompetition | Self::CancelCompetition => "competitionStop",
            Self::DisableEvent | Self::DeleteEvent => "automationEventDeactivate",
            Self::DisableAction => "automationActionDeactivate",
            Self::EnableEvent | Self::EnableAction => "automationReusableActivation",
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft => "automationDrafts",
            Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => "automationReusableWrite",
        }
    }
    pub fn capability(self) -> tiktools_collaboration::Capability {
        match self {
            Self::Disable => tiktools_collaboration::Capability::WorkflowsToggle,
            Self::Delete => tiktools_collaboration::Capability::WorkflowsEdit,
            Self::AdjustPoints => tiktools_collaboration::Capability::PointsAdjust,
            Self::StartCompetition
            | Self::ResumeCompetition
            | Self::PauseCompetition
            | Self::CancelCompetition => tiktools_collaboration::Capability::CompetitionsManage,
            Self::DisableEvent | Self::DisableAction | Self::EnableEvent | Self::EnableAction => {
                tiktools_collaboration::Capability::AutomationToggle
            }
            Self::DeleteEvent => tiktools_collaboration::Capability::AutomationEdit,
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft
            | Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => tiktools_collaboration::Capability::AutomationEdit,
        }
    }
    pub fn kind(self) -> tiktools_collaboration::ResourceKind {
        match self {
            Self::AdjustPoints => tiktools_collaboration::ResourceKind::Viewer,
            Self::StartCompetition
            | Self::ResumeCompetition
            | Self::PauseCompetition
            | Self::CancelCompetition => tiktools_collaboration::ResourceKind::Competition,
            Self::DisableEvent
            | Self::DisableAction
            | Self::EnableEvent
            | Self::EnableAction
            | Self::DeleteEvent
            | Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft
            | Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => tiktools_collaboration::ResourceKind::Automation,
            Self::Disable | Self::Delete => tiktools_collaboration::ResourceKind::Workflow,
        }
    }
    pub fn scope(self) -> &'static str {
        if matches!(
            self,
            Self::EnableAction
                | Self::DisableAction
                | Self::CreateActionDraft
                | Self::UpdateActionDraft
                | Self::CreateActionActive
                | Self::UpdateActionActive
        ) {
            "action"
        } else if self.is_event()
            || matches!(
                self,
                Self::CreateEventDraft
                    | Self::UpdateEventDraft
                    | Self::CreateEventActive
                    | Self::UpdateEventActive
            )
        {
            "event"
        } else {
            "host"
        }
    }
    pub fn is_event(self) -> bool {
        matches!(
            self,
            Self::DisableEvent | Self::DeleteEvent | Self::EnableEvent
        )
    }
    pub fn is_competition(self) -> bool {
        matches!(
            self,
            Self::StartCompetition
                | Self::ResumeCompetition
                | Self::PauseCompetition
                | Self::CancelCompetition
        )
    }
    pub fn is_activation(self) -> bool {
        matches!(self, Self::EnableEvent | Self::EnableAction)
    }
    pub fn is_automation_command(self) -> bool {
        self.is_event() || self.is_activation() || self == Self::DisableAction
    }
    pub fn is_delete(self) -> bool {
        matches!(self, Self::Delete | Self::DeleteEvent)
    }
    pub fn is_proposal(self) -> bool {
        matches!(
            self,
            Self::CreateEventDraft
                | Self::UpdateEventDraft
                | Self::CreateActionDraft
                | Self::UpdateActionDraft
                | Self::CreateEventActive
                | Self::UpdateEventActive
                | Self::CreateActionActive
                | Self::UpdateActionActive
        )
    }
    pub fn is_active_proposal(self) -> bool {
        matches!(
            self,
            Self::CreateEventActive
                | Self::UpdateEventActive
                | Self::CreateActionActive
                | Self::UpdateActionActive
        )
    }
    pub fn proposal_family(self) -> Option<tiktools_collaboration::AutomationFamily> {
        if self.is_proposal() {
            tiktools_collaboration::AutomationFamily::from_scope(self.scope())
        } else {
            None
        }
    }
    pub(super) fn supported_in(self, version: u32) -> bool {
        match self {
            Self::Disable | Self::Delete => matches!(version, 1..=VERSION),
            Self::AdjustPoints => matches!(version, 2..=VERSION),
            Self::DisableEvent | Self::DeleteEvent => matches!(version, 3..=VERSION),
            Self::EnableEvent | Self::EnableAction => matches!(version, 5..=VERSION),
            Self::DisableAction => matches!(version, 9..=VERSION),
            Self::CreateEventDraft
            | Self::UpdateEventDraft
            | Self::CreateActionDraft
            | Self::UpdateActionDraft => matches!(version, 4..=VERSION),
            Self::CreateEventActive
            | Self::UpdateEventActive
            | Self::CreateActionActive
            | Self::UpdateActionActive => matches!(version, 6..=VERSION),
            Self::PauseCompetition | Self::CancelCompetition => matches!(version, 7..=VERSION),
            Self::StartCompetition | Self::ResumeCompetition => matches!(version, 8..=VERSION),
        }
    }
}
