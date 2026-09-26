pub trait IdSource {
    fn next_id(&self) -> String;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UuidV7Ids;

impl IdSource for UuidV7Ids {
    fn next_id(&self) -> String {
        uuid::Uuid::now_v7().to_string()
    }
}
