use rs_merkle::{algorithms::Keccak256, MerkleProof, MerkleTree};

struct Keccak256Wrapper(Keccak256);

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

    pub fn commit(&mut self) -> (bool, Vec<u8>, Vec<usize>) {
        self.tree.commit();

        let leaves = self.get_leaves();
        if leaves.is_empty() {
            return (true, vec![], vec![]);
        }

        let indices_to_prove: Vec<usize> = (0..leaves.len()).collect();
        let proof_result = self.tree.proof(&indices_to_prove);
        let proof_bytes = proof_result.to_bytes();

        let root_opt = self.tree.root();
        if root_opt.is_none() {
            eprintln!(
                "Tree '{}': Root is None after commit. Rolling back.",
                self.identifier
            );
            self.rollback();
            return (false, vec![], vec![]);
        }
        let root = root_opt.unwrap();
        let proof_valid = proof_result.verify(root, &indices_to_prove, &leaves, leaves.len());

        if proof_valid {
            return (true, proof_bytes, indices_to_prove);
        } else {
            eprintln!(
                "Tree '{}': Proof verification FAILED after commit. Rolling back.",
                self.identifier
            );
            self.rollback();
            return (false, vec![], vec![]);
        }
    }

    pub fn get_leaves(&self) -> Vec<[u8; 32]> {
        self.tree.leaves().unwrap_or_default()
    }

    pub fn generate_proof_bytes(&self, indices: &[usize]) -> Vec<u8> {
        self.tree.proof(indices).to_bytes()
    }

    pub fn rollback(&mut self) {
        self.tree.rollback();
        println!("Tree '{}': Rolled back.", self.identifier);
    }

    pub fn verify_proof_bytes(
        &self,
        leaves_to_verify: &[[u8; 32]],
        indices: &[usize],
        proof_bytes: &[u8],
    ) -> bool {
        let root = match self.tree.root() {
            Some(r) => r,
            None => return false,
        };
        let total_leaves = self.tree.leaves().map_or(0, |l| l.len());

        match MerkleProof::<Keccak256>::from_bytes(proof_bytes) {
            Ok(proof) => proof.verify(root, indices, leaves_to_verify, total_leaves),
            Err(_) => false,
        }
    }

    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.tree.root()
    }

    pub fn verify_root(&self, claimed_root: [u8; 32]) -> bool {
        self.tree
            .root()
            .map_or(false, |actual_root| actual_root == claimed_root)
    }
}
