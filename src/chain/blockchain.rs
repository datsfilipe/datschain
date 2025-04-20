use crate::chain::block::Block;

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub current_difficulty_bits: u32,
}

impl Blockchain {
    pub fn new(current_difficulty_bits: u32) -> Self {
        let genesis_block = Block::new(vec![], vec![], 0);

        Self {
            blocks: vec![genesis_block],
            current_difficulty_bits,
        }
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks.get(height as usize)
    }
}
