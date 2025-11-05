use redb::{Database, TableDefinition, ReadableTable};
use anyhow::Result;

const PACKAGES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("packages");
const FILES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("files");
const REFCOUNT_TABLE: TableDefinition<&str, u64> = TableDefinition::new("refcounts");

pub struct StoreDatabase {
    db: Database,
}

impl StoreDatabase {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Database::create(path)?;
        Ok(Self { db })
    }
    
    pub fn insert_package(&self, hash: &str, metadata: &PackageMetadata) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PACKAGES_TABLE)?;
            let data = bincode::serialize(metadata)?;
            table.insert(hash, data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
    
    pub fn increment_refcount(&self, hash: &str) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(REFCOUNT_TABLE)?;
            let current = table.get(hash)?.map(|v| v.value()).unwrap_or(0);
            table.insert(hash, current + 1)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
