use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GridErrors {
    OutOfBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Grid<T> {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<T>,
}

impl<T> Grid<T> {
    pub fn new(width: usize, height: usize, cells: Vec<T>) -> Grid<T> {
        if cells.len() != width * height {
            panic!("Cell size does not match input width * height");
        }

        Grid {
            width,
            height,
            cells,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(&self.cells[y * self.width + x])
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(&mut self.cells[y * self.width + x])
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<(), GridErrors> {
        if x >= self.width || y >= self.height {
            return Err(GridErrors::OutOfBounds);
        }

        self.cells[y * self.width + x] = value;

        Ok(())
    }

    pub fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    pub fn flat_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn index(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }

    pub fn size(&self) -> usize {
        self.width * self.height
    }
}

impl<T: Clone> Grid<T> {
    pub fn filled(width: usize, height: usize, value: T) -> Grid<T> {
        Grid {
            width,
            height,
            cells: vec![value; width * height],
        }
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        if index.0 >= self.width || index.1 >= self.height {
            panic!("Index {}, {} is out of bounds.", index.0, index.1)
        }

        &self.cells[index.1 * self.width + index.0]
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        if index.0 >= self.width || index.1 >= self.height {
            panic!("Index {}, {} is out of bounds.", index.0, index.1)
        }

        &mut self.cells[index.1 * self.width + index.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn preserves_dimensions_and_cells() {
            let grid = Grid::new(3, 2, vec![1, 2, 3, 4, 5, 6]);

            assert_eq!(grid.width, 3);
            assert_eq!(grid.height, 2);
            assert_eq!(grid[(0, 0)], 1);
            assert_eq!(grid[(2, 1)], 6);
        }

        #[test]
        #[should_panic(expected = "Cell size does not match input width * height")]
        fn rejects_incorrect_cell_count() {
            Grid::new(3, 2, vec![1, 2, 3]);
        }
    }

    mod filled {
        use super::*;

        #[test]
        fn initializes_all_cells() {
            let grid = Grid::filled(3, 2, false);

            assert_eq!(grid.width, 3);
            assert_eq!(grid.height, 2);
            assert_eq!(grid.cells, vec![false; 6]);
        }
    }

    mod get {
        use super::*;

        #[test]
        fn returns_cells_and_handles_out_of_bounds() {
            let grid = Grid::new(3, 2, vec![1, 2, 3, 4, 5, 6]);

            assert_eq!(grid.get(0, 0), Some(&1));
            assert_eq!(grid.get(2, 1), Some(&6));
            assert_eq!(grid.get(3, 0), None);
            assert_eq!(grid.get(0, 2), None);
        }
    }

    mod get_mut {
        use super::*;

        #[test]
        fn updates_cells_and_handles_out_of_bounds() {
            let mut grid = Grid::filled(3, 2, 0);

            *grid.get_mut(1, 1).unwrap() = 7;

            assert_eq!(grid[(1, 1)], 7);
            assert!(grid.get_mut(3, 1).is_none());
        }
    }

    mod set {
        use super::*;

        #[test]
        fn updates_cells_and_rejects_out_of_bounds() {
            let mut grid = Grid::filled(3, 2, 0);

            assert_eq!(grid.set(2, 1, 7), Ok(()));
            assert_eq!(grid[(2, 1)], 7);
            assert_eq!(grid.set(3, 1, 9), Err(GridErrors::OutOfBounds));
            assert_eq!(grid.set(2, 2, 9), Err(GridErrors::OutOfBounds));
            assert_eq!(grid[(2, 1)], 7);
        }
    }

    mod in_bounds {
        use super::*;

        #[test]
        fn checks_both_dimensions() {
            let grid = Grid::filled(3, 2, 0);

            assert!(grid.in_bounds(0, 0));
            assert!(grid.in_bounds(2, 1));
            assert!(!grid.in_bounds(3, 0));
            assert!(!grid.in_bounds(0, 2));
        }
    }

    mod flat_index {
        use super::*;

        #[test]
        fn uses_row_major_order() {
            let grid = Grid::filled(3, 2, 0);

            assert_eq!(grid.flat_index(0, 0), 0);
            assert_eq!(grid.flat_index(2, 0), 2);
            assert_eq!(grid.flat_index(0, 1), 3);
            assert_eq!(grid.flat_index(2, 1), 5);
        }
    }

    mod index {
        use super::*;

        #[test]
        fn supports_read_and_write() {
            let mut grid = Grid::filled(3, 2, 0);

            grid[(1, 1)] = 42;

            assert_eq!(grid[(1, 1)], 42);
            assert_eq!(grid[(0, 1)], 0);
        }

        #[test]
        #[should_panic(expected = "Index 3, 0 is out of bounds.")]
        fn out_of_bounds_panics() {
            let grid = Grid::filled(3, 2, 0);
            let _ = grid[(3, 0)];
        }
    }

    mod index_mut {
        use super::*;

        #[test]
        #[should_panic(expected = "Index 0, 2 is out of bounds.")]
        fn out_of_bounds_panics() {
            let mut grid = Grid::filled(3, 2, 0);
            grid[(0, 2)] = 1;
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn copies_cell_storage() {
            let original = Grid::filled(3, 2, 1);
            let mut cloned = original.clone();

            cloned[(1, 1)] = 9;

            assert_eq!(original[(1, 1)], 1);
            assert_eq!(cloned[(1, 1)], 9);
        }
    }
}
