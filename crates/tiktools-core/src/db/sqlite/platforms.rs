//! Additive identity and analytics schema; existing balances remain untouched.
use super::{DatabaseError, DatabaseManager};
use rusqlite::params;
use tiktools_live::LiveEnvelope;

impl DatabaseManager {
    pub(crate) fn record_platform_activity(
        &self,
        event: &LiveEnvelope,
    ) -> Result<(), DatabaseError> {
        let mut db = self.open(&self.points_path())?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(user) = &event.user {
            let key = user.key();
            if !crate::services::valid_viewer_id(&key) {
                return Err(DatabaseError::Invalid(
                    "Platform viewer identity exceeds the points limit".into(),
                ));
            }
            // Never attach an existing legacy row to an unrelated provider.
            let collision:i64=tx.query_row("SELECT EXISTS(SELECT 1 FROM viewers v WHERE v.unique_id=?1 AND NOT EXISTS(SELECT 1 FROM platform_viewers p WHERE p.storage_key=v.unique_id AND p.platform=?2 AND p.user_id=?3))",params![key,user.platform.as_str(),user.id],|r|r.get(0))?;
            if collision != 0 {
                return Err(DatabaseError::Invalid("Viewer identity collision".into()));
            }
            tx.execute("INSERT INTO platform_viewers(platform,user_id,storage_key,handle,display_name) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(platform,user_id) DO UPDATE SET handle=excluded.handle,display_name=excluded.display_name",params![user.platform.as_str(),user.id,key,user.handle,user.display_name])?;
        }
        tx.execute("INSERT INTO platform_channels(platform,channel_id,handle,room_id) VALUES (?1,?2,?3,?4) ON CONFLICT(platform,channel_id) DO UPDATE SET handle=excluded.handle,room_id=excluded.room_id",params![event.platform.as_str(),event.channel.id,event.channel.handle,event.channel.room_id])?;
        let received = i64::try_from(event.received_at)
            .map_err(|_| DatabaseError::Invalid("Event timestamp is out of range".into()))?;
        let day = received / 86_400_000;
        tx.execute("INSERT INTO platform_analytics(platform,channel_id,day,event_type,event_count,last_received_at) VALUES (?1,?2,?3,?4,1,?5) ON CONFLICT(platform,channel_id,day,event_type) DO UPDATE SET event_count=event_count+1,last_received_at=MAX(last_received_at,excluded.last_received_at)",params![event.platform.as_str(),event.channel.id,day,event.payload.event_type(),received])?;
        tx.commit()?;
        Ok(())
    }
    pub fn platform_analytics(&self) -> Result<Vec<serde_json::Value>, DatabaseError> {
        let db = self.open(&self.points_path())?;
        let mut query=db.prepare("SELECT platform,channel_id,day,event_type,event_count,last_received_at FROM platform_analytics ORDER BY day DESC,platform,channel_id,event_type LIMIT 1000")?;
        let rows=query.query_map([],|r|Ok(serde_json::json!({"platform":r.get::<_,String>(0)?,"channelId":r.get::<_,String>(1)?,"day":r.get::<_,i64>(2)?,"eventType":r.get::<_,String>(3)?,"count":r.get::<_,i64>(4)?,"lastReceivedAt":r.get::<_,i64>(5)?})))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
