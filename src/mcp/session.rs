use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub created_at: i64,
    pub last_active: i64,
}

impl Session {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_active: now,
        }
    }

    pub fn with_id(id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Self {
            id,
            created_at: now,
            last_active: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_active = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
    }

    pub fn is_expired(&self, ttl_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        now - self.last_active > ttl_secs as i64
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    max_sessions: usize,
    ttl_secs: u64,
}

impl SessionManager {
    pub fn new(max_sessions: usize, ttl_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            max_sessions,
            ttl_secs,
        }
    }

    pub fn create(&mut self) -> Session {
        if self.sessions.len() >= self.max_sessions {
            self.cleanup_expired();
        }

        if self.sessions.len() >= self.max_sessions {
            // Remove oldest session to make room
            if let Some(oldest) = self.sessions.iter()
                .min_by_key(|(_, s)| s.last_active)
                .map(|(k, _)| k.clone())
            {
                self.sessions.remove(&oldest);
            }
        }

        let session = Session::new();
        let id = session.id.clone();
        self.sessions.insert(id, session.clone());
        session
    }

    pub fn get_or_create(&mut self, session_id: Option<&str>) -> Session {
        if let Some(id) = session_id {
            if let Some(session) = self.sessions.get_mut(id) {
                session.touch();
                return session.clone();
            }
        }
        self.create()
    }

    pub fn touch(&mut self, session_id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.touch();
            return true;
        }
        false
    }

    pub fn remove(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    pub fn get(&self, session_id: &str) -> Option<Session> {
        self.sessions.get(session_id).cloned()
    }

    pub fn cleanup_expired(&mut self) -> Vec<String> {
        let expired: Vec<String> = self.sessions.iter()
            .filter(|(_, s)| s.is_expired(self.ttl_secs))
            .map(|(k, _)| k.clone())
            .collect();

        for id in &expired {
            self.sessions.remove(id);
        }

        expired
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}

pub type SessionManagerHandle = Arc<Mutex<SessionManager>>;

pub fn create_session_manager(max_sessions: usize, ttl_secs: u64) -> SessionManagerHandle {
    Arc::new(Mutex::new(SessionManager::new(max_sessions, ttl_secs)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert_eq!(session.created_at, session.last_active);
    }

    #[test]
    fn test_session_manager() {
        let mut mgr = SessionManager::new(10, 60);
        let session = mgr.create();
        assert!(mgr.touch(&session.id));
        assert!(!mgr.remove(&session.id));
        assert!(!mgr.touch(&session.id)); // already removed
    }
}