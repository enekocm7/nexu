#[derive(Clone)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_connection: Option<u64>,
    pub last_message: Option<String>,
}

impl PartialEq for Contact {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
