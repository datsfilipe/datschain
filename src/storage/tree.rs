use rs_merkle::{algorithms::Keccak256, MerkleProof, MerkleTree};

pub struct Tree {
    identifier: String,
    tree: MerkleTree<Keccak256>,
}

fn is_equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    for i in 0..32 {
        if a[i] != b[i] {
            return false;
        }
    }
    true
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

    pub fn proof(&self, leaves: Vec<[u8; 32]>) -> bool {
        let root = self.tree.root().ok_or(false);
        let stored_leaves = self.tree.leaves().ok_or(false);
        if stored_leaves.is_err() {
            return false;
        }
        let stored_leaves = stored_leaves.unwrap();

        let mut indices = Vec::new();
        for leaf in &leaves {
            if !stored_leaves.contains(&leaf) {
                return false;
            }
            indices.push(
                stored_leaves
                    .iter()
                    .position(|x| is_equal(x, &leaf))
                    .unwrap(),
            );
        }

        let proof = MerkleProof::<Keccak256>::try_from(self.tree.proof(&indices).to_bytes());
        match proof {
            Ok(proof) => proof.verify(root.unwrap(), &indices, &leaves, leaves.len()),
            Err(_) => false,
        }
    }
}
