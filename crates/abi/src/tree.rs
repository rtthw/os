//! # Tree

use std::collections::HashMap;



#[derive(Debug)]
pub struct Tree<T> {
    roots: HashMap<u64, Node<T>>,
    branches: HashMap<u64, Option<u64>>,
}

#[derive(Debug)]
struct Node<T> {
    id: u64,
    element: T,
    leaves: HashMap<u64, Node<T>>,
}

#[derive(Clone, Copy, Debug)]
pub struct NodeRef<'tree, T> {
    pub branch_id: Option<u64>,
    pub element: &'tree T,
    pub leaves: LeavesRef<'tree, T>,
}

#[derive(Debug)]
pub struct NodeMut<'tree, T> {
    pub branch_id: Option<u64>,
    pub element: &'tree mut T,
    pub leaves: LeavesMut<'tree, T>,
}

#[derive(Clone, Copy, Debug)]
pub struct LeavesRef<'tree, T> {
    branch_id: Option<u64>,
    leaves: &'tree HashMap<u64, Node<T>>,
    branches: BranchesRef<'tree>,
}

#[derive(Debug)]
pub struct LeavesMut<'tree, T> {
    branch_id: Option<u64>,
    leaves: &'tree mut HashMap<u64, Node<T>>,
    branches: BranchesMut<'tree>,
}

#[derive(Clone, Copy, Debug)]
pub struct BranchesRef<'tree> {
    branches: &'tree HashMap<u64, Option<u64>>,
}

#[derive(Debug)]
pub struct BranchesMut<'tree> {
    branches: &'tree mut HashMap<u64, Option<u64>>,
}

impl<T> Tree<T> {
    pub fn new() -> Self {
        Self {
            roots: HashMap::new(),
            branches: HashMap::new(),
        }
    }

    pub fn roots(&self) -> LeavesRef<'_, T> {
        LeavesRef {
            branch_id: None,
            leaves: &self.roots,
            branches: BranchesRef {
                branches: &self.branches,
            },
        }
    }

    pub fn roots_mut(&mut self) -> LeavesMut<'_, T> {
        LeavesMut {
            branch_id: None,
            leaves: &mut self.roots,
            branches: BranchesMut {
                branches: &mut self.branches,
            },
        }
    }

    pub fn root_ids(&self) -> impl Iterator<Item = u64> {
        self.roots.keys().copied()
    }

    pub fn find(&self, id: impl Into<u64>) -> Option<NodeRef<'_, T>> {
        self.roots()._find(id.into())
    }

    pub fn find_mut(&mut self, id: impl Into<u64>) -> Option<NodeMut<'_, T>> {
        self.roots_mut()._find_mut(id.into())
    }
}

impl<T> Node<T> {
    fn as_ref<'tree>(
        &'tree self,
        branch_id: Option<u64>,
        branch_map: &'tree HashMap<u64, Option<u64>>,
    ) -> NodeRef<'tree, T> {
        NodeRef {
            branch_id,
            element: &self.element,
            leaves: LeavesRef {
                branch_id: Some(self.id),
                leaves: &self.leaves,
                branches: BranchesRef {
                    branches: branch_map,
                },
            },
        }
    }

    fn as_mut<'tree>(
        &'tree mut self,
        branch_id: Option<u64>,
        branch_map: &'tree mut HashMap<u64, Option<u64>>,
    ) -> NodeMut<'tree, T> {
        NodeMut {
            branch_id,
            element: &mut self.element,
            leaves: LeavesMut {
                branch_id: Some(self.id),
                leaves: &mut self.leaves,
                branches: BranchesMut {
                    branches: branch_map,
                },
            },
        }
    }
}

impl<'tree, T> LeavesRef<'tree, T> {
    fn _find(self, id: u64) -> Option<NodeRef<'tree, T>> {
        let branch_id = self.branches.branches.get(&id)?;

        let id_path = if let Some(branch_id) = branch_id {
            self.branches.get_id_path(*branch_id, self.branch_id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_leaves = self.leaves;
        while let Some((branch_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_leaves = &node_leaves.get(branch_id)?.leaves;
        }

        let node = node_leaves.get(&id)?;

        Some(node.as_ref(*branch_id, self.branches.branches))
    }
}

impl<'tree, T> LeavesMut<'tree, T> {
    pub fn insert(&mut self, leaf_id: impl Into<u64>, value: T) -> NodeMut<'_, T> {
        let leaf_id = leaf_id.into();

        assert!(
            !self.branches.branches.contains_key(&leaf_id),
            "already present"
        );

        self.branches.branches.insert(leaf_id, self.branch_id);
        self.leaves.insert(
            leaf_id,
            Node {
                id: leaf_id,
                element: value,
                leaves: HashMap::new(),
            },
        );

        self.leaves
            .get_mut(&leaf_id)
            .unwrap()
            .as_mut(self.branch_id, self.branches.branches)
    }

    pub fn remove(&mut self, leaf_id: impl Into<u64>) -> Option<T> {
        let leaf_id = leaf_id.into();
        let leaf = self.leaves.remove(&leaf_id)?;

        fn remove_leaves<U>(node: &Node<U>, branches: &mut HashMap<u64, Option<u64>>) {
            for leaf in &node.leaves {
                remove_leaves(leaf.1, branches);
            }
            branches.remove(&node.id);
        }

        remove_leaves(&leaf, self.branches.branches);

        Some(leaf.element)
    }

    fn _find_mut(self, id: u64) -> Option<NodeMut<'tree, T>> {
        let branch_id = self.branches.branches.get(&id).copied()?;

        let id_path = if let Some(branch_id) = branch_id {
            self.branches.get_id_path(branch_id, self.branch_id)
        } else {
            Vec::new()
        };

        let mut id_path = id_path.as_slice();
        let mut node_leaves: &'tree mut _ = &mut *self.leaves;
        while let Some((branch_id, new_id_path)) = id_path.split_last() {
            id_path = new_id_path;
            node_leaves = &mut node_leaves.get_mut(branch_id)?.leaves;
        }

        let node = node_leaves.get_mut(&id)?;

        Some(node.as_mut(branch_id, &mut *self.branches.branches))
    }
}

impl BranchesRef<'_> {
    pub fn get_id_path(self, id: u64, start_id: Option<u64>) -> Vec<u64> {
        let mut path = Vec::new();

        if !self.branches.contains_key(&id) {
            return path;
        }

        let mut current_id = Some(id);
        while let Some(current) = current_id {
            path.push(current);
            current_id = *self
                .branches
                .get(&current)
                .expect("All IDs in the tree should have a branch in the branch map");
            if current_id == start_id {
                break;
            }
        }

        if current_id != start_id {
            path.clear();
        }

        path
    }
}

impl BranchesMut<'_> {
    pub fn get_id_path(&self, id: u64, start_id: Option<u64>) -> Vec<u64> {
        BranchesRef {
            branches: self.branches,
        }
        .get_id_path(id, start_id)
    }
}
