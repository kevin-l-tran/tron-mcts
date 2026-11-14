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
