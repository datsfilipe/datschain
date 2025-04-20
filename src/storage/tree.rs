use rs_merkle::{algorithms::Keccak256, MerkleProof, MerkleTree};

#[allow(dead_code)]
pub struct Tree {
    identifier: String,
    tree: MerkleTree<Keccak256>,
}

impl Tree {
    pub fn new(identifier: String) -> Self {
        Tree {
            identifier,
            tree: MerkleTree::<Keccak256>::new(),
        }
    }

    pub fn insert(&mut self, value: [u8; 32]) {
        self.tree.insert(value);
    }

    pub fn commit(&mut self) {
        self.tree.commit();
    }

    pub fn get_leaves(&self) -> Vec<[u8; 32]> {
        match self.tree.leaves() {
            Some(leaves) => leaves,
            None => vec![],
        }
    }

    pub fn generate_proof_bytes(&self, indices: &[usize]) -> Vec<u8> {
        self.tree.proof(indices).to_bytes()
    }

    pub fn verify_proof_bytes(
        &self,
        leaves: &[[u8; 32]],
        indices: &[usize],
        proof_bytes: &[u8],
    ) -> bool {
        let root = match self.tree.root() {
            Some(r) => r,
            None => return false,
        };
        // real total leaf count at proof time:
        let total_leaves = match self.tree.leaves() {
            Some(ref v) => v.len(),
            None => return false,
        };
        if let Ok(proof) = MerkleProof::<Keccak256>::from_bytes(proof_bytes) {
            return proof.verify(root, indices, leaves, total_leaves);
        }
        false
    }
}
