use ctx_core::{OrderingStrategy, Pack, RenderPolicy};
use ctx_sources::{SourceHandlerRegistry, SourceOptions};
use ctx_storage::Storage;

#[tokio::test]
async fn test_pack_lifecycle() {
    // Create a temporary database
    let temp_dir = std::env::temp_dir().join(format!("ctx-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let storage = Storage::new(Some(db_path.clone())).await.unwrap();

    // Create pack
    let policies = RenderPolicy {
        budget_tokens: 100000,
        ordering: OrderingStrategy::PriorityThenTime,
    };
    let pack = Pack::new("test-pack".to_string(), policies);
    storage.create_pack(&pack).await.unwrap();

    // List packs
    let packs = storage.list_packs().await.unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].name, "test-pack");

    // Get pack by name
    let retrieved_pack = storage.get_pack_by_name("test-pack").await.unwrap();
    assert_eq!(retrieved_pack.id, pack.id);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[tokio::test]
async fn test_artifact_operations() {
    // Create a temporary database
    let temp_dir = std::env::temp_dir().join(format!("ctx-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let storage = Storage::new(Some(db_path.clone())).await.unwrap();
    let registry = SourceHandlerRegistry::new();

    // Create pack
    let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
    storage.create_pack(&pack).await.unwrap();

    // Create a test file
    let test_file = temp_dir.join("test.txt");
    std::fs::write(&test_file, "Hello, world!").unwrap();

    // Parse and add artifact
    let options = SourceOptions::default();
    let artifact = registry
        .parse(&format!("file:{}", test_file.display()), options)
        .await
        .unwrap();

    storage.create_artifact(&artifact).await.unwrap();
    storage
        .add_artifact_to_pack(&pack.id, &artifact.id, 0)
        .await
        .unwrap();

    // Get pack artifacts
    let pack_items = storage.get_pack_artifacts(&pack.id).await.unwrap();
    assert_eq!(pack_items.len(), 1);
    assert_eq!(pack_items[0].artifact.id, artifact.id);

    // Remove artifact
    storage
        .remove_artifact_from_pack(&pack.id, &artifact.id)
        .await
        .unwrap();

    let pack_items = storage.get_pack_artifacts(&pack.id).await.unwrap();
    assert_eq!(pack_items.len(), 0);

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[tokio::test]
async fn test_text_handler() {
    let registry = SourceHandlerRegistry::new();

    let options = SourceOptions::default();
    let artifact = registry
        .parse("text:Hello, world!", options)
        .await
        .unwrap();

    let content = registry.load(&artifact).await.unwrap();
    assert_eq!(content, "Hello, world!");
}
