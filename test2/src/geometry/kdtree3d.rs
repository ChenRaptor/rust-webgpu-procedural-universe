use glam::Vec3;

#[derive(Clone)]
struct KDNode {
    point: Vec3,
    index: usize,
    left: Option<usize>,  // Indice du fils gauche dans le vecteur, ou None si nul
    right: Option<usize>, // Indice du fils droit dans le vecteur, ou None si nul
}

impl KDNode {
    fn new(point: Vec3, index: usize) -> Self {
        Self {
            point,
            index,
            left: None,
            right: None,
        }
    }
}

pub struct KDTree3D {
    nodes: Vec<KDNode>,
    root_index: Option<usize>,
}

impl KDTree3D {
    pub fn new(points: &[Vec3]) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            root_index: None,
        };

        if !points.is_empty() {
            let mut indices: Vec<usize> = (0..points.len()).collect();
            tree.root_index = tree.build(points, &mut indices, 0, points.len(), 0);
        }

        tree
    }

    pub fn nearest_neighbor(&self, target: Vec3) -> usize {
        let mut best_index = 0;
        let mut best_dist_sq = f32::MAX;
        
        if let Some(root) = self.root_index {
            self.nearest(root, target, 0, &mut best_index, &mut best_dist_sq);
        }
        
        best_index
    }

    fn build(
        &mut self,
        points: &[Vec3],
        indices: &mut [usize],
        start: usize,
        end: usize,
        depth: usize,
    ) -> Option<usize> {
        if start >= end {
            return None;
        }

        let axis = depth % 3;

        // Fonction de comparaison selon l'axe
        let comp = |a: &usize, b: &usize| -> std::cmp::Ordering {
            let point_a = points[*a];
            let point_b = points[*b];
            
            let val_a = match axis {
                0 => point_a.x,
                1 => point_a.y,
                _ => point_a.z,
            };
            
            let val_b = match axis {
                0 => point_b.x,
                1 => point_b.y,
                _ => point_b.z,
            };
            
            val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
        };

        let median = (start + end) / 2;
        
        // Équivalent de std::nth_element en Rust
        indices[start..end].select_nth_unstable_by(median - start, comp);

        let median_index = indices[median];
        let mut node = KDNode::new(points[median_index], median_index);

        // Construire les sous-arbres
        node.left = self.build(points, indices, start, median, depth + 1);
        node.right = self.build(points, indices, median + 1, end, depth + 1);

        // Ajouter le nœud au vecteur et retourner son index
        self.nodes.push(node);
        Some(self.nodes.len() - 1)
    }

    fn nearest(
        &self,
        node_idx: usize,
        target: Vec3,
        depth: usize,
        best_index: &mut usize,
        best_dist_sq: &mut f32,
    ) {
        let node = &self.nodes[node_idx];
        let dist_sq = (node.point - target).length_squared();

        if dist_sq < *best_dist_sq {
            *best_dist_sq = dist_sq;
            *best_index = node.index;
        }

        let axis = depth % 3;
        let diff = match axis {
            0 => target.x - node.point.x,
            1 => target.y - node.point.y,
            _ => target.z - node.point.z,
        };

        let (near_child, far_child) = if diff < 0.0 {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };

        // Parcourir le côté proche en premier
        if let Some(near_idx) = near_child {
            self.nearest(near_idx, target, depth + 1, best_index, best_dist_sq);
        }

        // Vérifier si on doit parcourir le côté éloigné
        if (diff * diff) < *best_dist_sq {
            if let Some(far_idx) = far_child {
                self.nearest(far_idx, target, depth + 1, best_index, best_dist_sq);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdtree_basic() {
        let points = vec![
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
        ];

        let tree = KDTree3D::new(&points);
        let nearest = tree.nearest_neighbor(Vec3::new(1.1, 2.1, 3.1));
        assert_eq!(nearest, 0); // Le premier point devrait être le plus proche
    }

    #[test]
    fn test_kdtree_empty() {
        let points = vec![];
        let tree = KDTree3D::new(&points);
        let nearest = tree.nearest_neighbor(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(nearest, 0); // Retourne 0 par défaut pour un arbre vide
    }
}
