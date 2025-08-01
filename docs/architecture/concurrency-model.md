# Concurrency Model

## Async Request Handling
- **Request Processing**: Concurrent handling of LSP requests
- **Resource Sharing**: Arc<Mutex<T>> for shared state
- **Blocking Operations**: Spawn blocking tasks for compiler calls
- **Backpressure**: Request queuing and cancellation support

## Synchronous Notifications
- **Document Updates**: Processed sequentially to maintain consistency
- **State Mutations**: Serialized access to document and symbol state
- **Event Ordering**: Maintain correct ordering of notifications

## Background Tasks
- **Indexing**: Background symbol indexing for large projects
- **Compilation**: Asynchronous compilation with result caching
- **File Watching**: Monitor file system changes for non-open files
