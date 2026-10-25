use serde::{Deserialize, Serialize};
use ttl_live_proto::webcast::model::base::{user::User, ImageModel};

/// Minimal, stable user identity shared by every public-area event.
///
/// Deliberately small: it holds only the fields that have been present across
/// every schema version we have seen, so consumers keep working when the
/// generated `User` layout shifts underneath.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventUser {
    pub id: u64,
    pub nickname: String,
    /// The `@handle`. Called `display_id` in the schema, `uniqueId` in the Node
    /// connector; exposed here under the connector's name.
    pub unique_id: String,
    pub sec_uid: String,
    /// Best available avatar URL from the generated user message.
    pub avatar_url: Option<String>,
}

impl EventUser {
    /// Normalises a generated `User`. A missing user yields the default, which
    /// keeps a malformed event usable rather than dropping it.
    pub(crate) fn normalize(user: Option<&User>) -> Self {
        let Some(user) = user else {
            return Self::default();
        };
        let avatar_url = [
            user.avatar_thumb.as_ref(),
            user.avatar_medium.as_ref(),
            user.avatar_large.as_ref(),
            user.avatar_jpg.as_ref(),
        ]
        .into_iter()
        .find_map(first_image_url);
        Self {
            id: user.id as u64,
            nickname: user.nickname.clone(),
            unique_id: user.display_id.clone(),
            sec_uid: user.sec_uid.clone(),
            avatar_url,
        }
    }

    /// Best available human-readable label, preferring the stable handle.
    pub fn label(&self) -> &str {
        if !self.unique_id.is_empty() {
            &self.unique_id
        } else if !self.nickname.is_empty() {
            &self.nickname
        } else {
            "unknown"
        }
    }
}

fn first_image_url(image: Option<&ImageModel>) -> Option<String> {
    image
        .and_then(|image| image.url_list.first())
        .filter(|url| !url.is_empty())
        .cloned()
}
