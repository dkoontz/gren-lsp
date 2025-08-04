use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use tokio::fs;
use tracing::{debug, info, warn};

/// Manages atomic file system operations with rollback capability
pub struct FileOperationManager {
    /// Operations staging area
    staging_dir: Option<PathBuf>,
}

/// Represents a transactional file operation context
pub struct FileTransaction {
    /// Transaction ID for logging
    transaction_id: String,
    /// Staged operations that can be committed or rolled back
    staged_operations: Vec<StagedOperation>,
}

/// A staged file operation that can be committed or rolled back
#[derive(Debug, Clone)]
enum StagedOperation {
    MoveFile {
        source: PathBuf,
        destination: PathBuf,
        backup_location: Option<PathBuf>,
    },
    CreateDirectory {
        path: PathBuf,
        created: bool,
    },
}

impl FileOperationManager {
    /// Create new file operation manager
    pub fn new() -> Self {
        Self {
            staging_dir: None,
        }  
    }

    /// Initialize staging directory for transactions
    async fn ensure_staging_dir(&mut self) -> Result<&PathBuf> {
        if self.staging_dir.is_none() {
            let temp_dir = std::env::temp_dir().join("gren-lsp-staging");
            fs::create_dir_all(&temp_dir).await
                .map_err(|e| anyhow!("Failed to create staging directory: {}", e))?;
            self.staging_dir = Some(temp_dir);
        }
        Ok(self.staging_dir.as_ref().unwrap())
    }

    /// Begin a new file operation transaction
    pub async fn begin_transaction(&self) -> Result<FileTransaction> {
        let transaction_id = uuid::Uuid::new_v4().to_string();
        debug!("Beginning file transaction: {}", transaction_id);

        Ok(FileTransaction {
            transaction_id,
            staged_operations: Vec::new(),
        })
    }

    /// Validate that a file operation can be performed
    pub async fn validate_file_operation(&self, source: &Path, destination: &Path) -> Result<()> {
        // Check source exists and is readable
        if !source.exists() {
            return Err(anyhow!("Source file does not exist: {}", source.display()));
        }

        let source_metadata = fs::metadata(source).await
            .map_err(|e| anyhow!("Cannot read source file metadata: {}", e))?;

        if !source_metadata.is_file() {
            return Err(anyhow!("Source is not a regular file: {}", source.display()));
        }

        // Check destination doesn't exist
        if destination.exists() {
            return Err(anyhow!("Destination file already exists: {}", destination.display()));
        }

        // Check destination parent directory exists or can be created and test permissions
        if let Some(parent) = destination.parent() {
            let parent_existed = parent.exists();
            
            if !parent_existed {
                // Try to create the parent directory
                fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!("Cannot create destination directory {}: {}", parent.display(), e))?;
            }
            
            // Test write permissions by creating a temporary file
            let test_file = parent.join(format!(".gren-lsp-test-{}", uuid::Uuid::new_v4()));
            match fs::write(&test_file, "test").await {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file).await;
                }
                Err(e) => {
                    // Clean up created directory if we created it
                    if !parent_existed {
                        let _ = fs::remove_dir_all(parent).await;
                    }
                    return Err(anyhow!("No write permission in destination directory: {}", e));
                }
            }
            
            // Clean up test directory if we created it
            if !parent_existed {
                let _ = fs::remove_dir_all(parent).await;
            }
        }

        Ok(())
    }
}

impl FileTransaction {
    /// Stage a file move operation
    pub async fn move_file(&mut self, source: &Path, destination: &Path) -> Result<()> {
        debug!("Staging file move: {} -> {}", source.display(), destination.display());

        // Validate the operation by creating a temporary manager for validation
        let temp_manager = FileOperationManager::new();
        temp_manager.validate_file_operation(source, destination).await?;

        // Create destination directory if needed
        if let Some(parent) = destination.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!("Failed to create destination directory: {}", e))?;
                
                self.staged_operations.push(StagedOperation::CreateDirectory {
                    path: parent.to_path_buf(),
                    created: true,
                });
            }
        }

        // Create backup location for rollback
        let backup_location = if source.exists() {
            Some(std::env::temp_dir().join(format!("gren-lsp-backup-{}-{}", 
                self.transaction_id,
                source.file_name().unwrap().to_string_lossy()
            )))
        } else {
            None
        };

        // Stage the move operation
        self.staged_operations.push(StagedOperation::MoveFile {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            backup_location,
        });

        Ok(())
    }

    /// Commit all staged operations atomically
    pub async fn commit(self) -> Result<()> {
        info!("Committing file transaction: {}", self.transaction_id);

        // Execute all staged operations
        for operation in &self.staged_operations {
            match operation {
                StagedOperation::MoveFile { source, destination, backup_location } => {
                    // Create backup if needed
                    if let Some(backup_path) = backup_location {
                        if source.exists() {
                            fs::copy(source, backup_path).await
                                .map_err(|e| anyhow!("Failed to create backup: {}", e))?;
                        }
                    }

                    // Perform the move
                    fs::rename(source, destination).await
                        .map_err(|e| anyhow!("Failed to move file: {}", e))?;

                    debug!("File moved: {} -> {}", source.display(), destination.display());
                }
                StagedOperation::CreateDirectory { path, created: _ } => {
                    // Directory was already created during staging
                    debug!("Directory confirmed: {}", path.display());
                }
            }
        }

        // Clean up backups after successful commit
        for operation in &self.staged_operations {
            if let StagedOperation::MoveFile { backup_location: Some(backup_path), .. } = operation {
                if backup_path.exists() {
                    let _ = fs::remove_file(backup_path).await;
                }
            }
        }

        info!("✅ File transaction committed successfully: {}", self.transaction_id);
        Ok(())
    }

    /// Roll back all staged operations
    pub async fn rollback(self) -> Result<()> {
        warn!("Rolling back file transaction: {}", self.transaction_id);

        // Reverse the operations
        for operation in self.staged_operations.iter().rev() {
            match operation {
                StagedOperation::MoveFile { source, destination, backup_location } => {
                    // If destination exists, remove it
                    if destination.exists() {
                        fs::remove_file(destination).await
                            .map_err(|e| anyhow!("Failed to remove destination during rollback: {}", e))?;
                    }

                    // Restore from backup if available
                    if let Some(backup_path) = backup_location {
                        if backup_path.exists() {
                            fs::rename(backup_path, source).await
                                .map_err(|e| anyhow!("Failed to restore from backup: {}", e))?;
                        }
                    }

                    debug!("File move rolled back: {} -> {}", destination.display(), source.display());
                }
                StagedOperation::CreateDirectory { path, created } => {
                    // Remove directory if we created it
                    if *created && path.exists() {
                        // Only remove if directory is empty
                        match fs::remove_dir(path).await {
                            Ok(_) => debug!("Directory removed during rollback: {}", path.display()),
                            Err(e) => debug!("Could not remove directory during rollback (may not be empty): {}", e),
                        }
                    }
                }
            }
        }

        info!("🔄 File transaction rolled back: {}", self.transaction_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_successful_file_move() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileOperationManager::new();

        // Create source file
        let source = temp_dir.path().join("source.txt");
        fs::write(&source, "test content").await.unwrap();

        let destination = temp_dir.path().join("destination.txt");

        // Test transaction
        let mut transaction = manager.begin_transaction().await.unwrap();
        transaction.move_file(&source, &destination).await.unwrap();
        transaction.commit().await.unwrap();

        // Verify results
        assert!(!source.exists());
        assert!(destination.exists());
        let content = fs::read_to_string(&destination).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_rollback_file_move() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileOperationManager::new();

        // Create source file
        let source = temp_dir.path().join("source.txt");
        fs::write(&source, "test content").await.unwrap();

        let destination = temp_dir.path().join("destination.txt");

        // Test transaction with rollback
        let mut transaction = manager.begin_transaction().await.unwrap();
        transaction.move_file(&source, &destination).await.unwrap();
        
        // Simulate failure by rolling back
        transaction.rollback().await.unwrap();

        // Verify rollback
        assert!(source.exists());
        assert!(!destination.exists());
        let content = fs::read_to_string(&source).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_validation_source_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileOperationManager::new();

        let source = temp_dir.path().join("nonexistent.txt");
        let destination = temp_dir.path().join("destination.txt");

        let result = manager.validate_file_operation(&source, &destination).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_validation_destination_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileOperationManager::new();

        // Create both files
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("destination.txt");
        fs::write(&source, "source").await.unwrap();
        fs::write(&destination, "destination").await.unwrap();

        let result = manager.validate_file_operation(&source, &destination).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}