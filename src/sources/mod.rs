pub mod deck_log;
pub mod holodelta;
pub mod holoduel;
pub mod tabletop_sim;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonCardEntry {
    pub card_number: String,
    pub rarity: u32,
    pub amount: u32,
}

#[derive(Debug, Clone)]
pub struct CommonDeck {
    pub deck_name: String,
    pub oshi: CommonCardEntry,
    pub main_deck: Vec<CommonCardEntry>,
    pub cheer_deck: Vec<CommonCardEntry>,
}
