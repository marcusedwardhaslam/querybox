pub mod profile;
pub mod storage;

use std::sync::Arc;

use crate::db::{
    mysql::MySqlDriver, postgres::PostgresDriver, sqlite::SqliteDriver, DatabaseDriver, DbError,
};
use profile::{ConnectionProfile, DatabaseEngine};

pub struct ConnectionManager {
    #[allow(dead_code)]
    pub profiles: Vec<ConnectionProfile>,
    active_driver: Option<Arc<dyn DatabaseDriver>>,
    pub active_profile: Option<ConnectionProfile>,
    pub active_database: Option<String>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let profiles = storage::load_profiles().unwrap_or_default();
        Self {
            profiles,
            active_driver: None,
            active_profile: None,
            active_database: None,
        }
    }

    pub fn driver(&self) -> Option<&dyn DatabaseDriver> {
        self.active_driver.as_deref()
    }

    pub fn driver_arc(&self) -> Option<Arc<dyn DatabaseDriver>> {
        self.active_driver.clone()
    }

    pub fn set_active_driver(
        &mut self,
        driver: Arc<dyn DatabaseDriver>,
        profile: ConnectionProfile,
    ) {
        self.active_driver = Some(driver);
        self.active_database = profile.default_database.clone();
        self.active_profile = Some(profile);
    }

    #[allow(dead_code)]
    pub async fn disconnect(&mut self) -> Result<(), DbError> {
        if let Some(driver) = self.active_driver.take() {
            driver.disconnect().await?;
        }
        self.active_profile = None;
        self.active_database = None;
        Ok(())
    }

    pub async fn connect_new(
        &mut self,
        profile: ConnectionProfile,
        password: &str,
    ) -> Result<(), DbError> {
        let driver: Arc<dyn DatabaseDriver> = match profile.engine {
            DatabaseEngine::MySql => Arc::new(MySqlDriver::connect(&profile, password).await?),
            DatabaseEngine::PostgreSql => {
                Arc::new(PostgresDriver::connect(&profile, password).await?)
            }
            DatabaseEngine::Sqlite => Arc::new(SqliteDriver::connect(&profile, password).await?),
        };
        self.set_active_driver(driver, profile);
        Ok(())
    }

    pub fn active_info(&self) -> Option<(String, String)> {
        self.active_profile
            .as_ref()
            .map(|p| (p.name.clone(), format!("{} • {}", p.engine, p.host)))
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), storage::StorageError> {
        storage::save_profiles(&self.profiles)
    }
}

pub async fn test_connect(profile: &ConnectionProfile, password: &str) -> Result<(), DbError> {
    match profile.engine {
        DatabaseEngine::MySql => MySqlDriver::connect(profile, password).await.map(|_| ()),
        DatabaseEngine::PostgreSql => PostgresDriver::connect(profile, password).await.map(|_| ()),
        DatabaseEngine::Sqlite => SqliteDriver::connect(profile, password).await.map(|_| ()),
    }
}
