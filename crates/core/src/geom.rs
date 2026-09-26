//! Grid geometry: positions, the four directions and a rectangular grid.

/// A tile position. Signed so neighbour maths can go negative before bounds
/// checks; `(0, 0)` is the top-left tile, `x` grows right, `y` grows down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Pos {
    /// Column.
    pub x: i32,
    /// Row.
    pub y: i32,
}

impl Pos {
    /// Creates a position.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan (4-way) distance between `a` and `b`.
    pub fn manhattan(a: Pos, b: Pos) -> u32 {
        a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
    }

    /// The adjacent position in direction `dir` (may be out of bounds).
    #[must_use]
    pub fn step(self, dir: Dir) -> Pos {
        let (dx, dy) = dir.delta();
        Pos::new(self.x + dx, self.y + dy)
    }
}

/// One of the four grid directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dir {
    /// `+x`.
    Right,
    /// `+y`.
    Down,
    /// `-x`.
    Left,
    /// `-y`.
    Up,
}

impl Dir {
    /// Every direction in the fixed order used for deterministic iteration.
    pub const ALL: [Dir; 4] = [Dir::Right, Dir::Down, Dir::Left, Dir::Up];

    /// The `(dx, dy)` offset of one step in this direction.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Up => (0, -1),
        }
    }
}

/// A `width × height` rectangle of cells, stored row-major. The cell count
/// always equals `width * height`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<T> {
    width: u16,
    height: u16,
    cells: Vec<T>,
}

impl<T> Grid<T> {
    /// Builds a grid from row-major `cells`; `None` if the count is not
    /// `width * height`.
    pub fn from_cells(width: u16, height: u16, cells: Vec<T>) -> Option<Self> {
        (cells.len() == usize::from(width) * usize::from(height)).then_some(Self {
            width,
            height,
            cells,
        })
    }

    /// A `width × height` grid with every cell set to `value`.
    pub fn filled(width: u16, height: u16, value: T) -> Self
    where
        T: Clone,
    {
        Self {
            width,
            height,
            cells: vec![value; usize::from(width) * usize::from(height)],
        }
    }

    /// Number of columns.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Every cell, row-major.
    pub fn cells(&self) -> &[T] {
        &self.cells
    }

    /// Whether `pos` is inside the grid.
    pub fn in_bounds(&self, pos: Pos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < i32::from(self.width) && pos.y < i32::from(self.height)
    }

    /// The cell at `pos`, or `None` if out of bounds.
    pub fn get(&self, pos: Pos) -> Option<&T> {
        self.cells.get(self.index(pos)?)
    }

    /// The cell at `pos` for writing, or `None` if out of bounds.
    pub fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        let i = self.index(pos)?;
        self.cells.get_mut(i)
    }

    /// The row-major index of `pos`, or `None` if out of bounds.
    fn index(&self, pos: Pos) -> Option<usize> {
        if !self.in_bounds(pos) {
            return None;
        }
        // In bounds, so both coordinates are non-negative and small.
        let x = usize::try_from(pos.x).ok()?;
        let y = usize::try_from(pos.y).ok()?;
        Some(y * usize::from(self.width) + x)
    }

    /// The in-bounds neighbours of `pos`, in [`Dir::ALL`] order.
    pub fn neighbors4(&self, pos: Pos) -> impl Iterator<Item = Pos> + '_ {
        Dir::ALL
            .into_iter()
            .map(move |d| pos.step(d))
            .filter(|&p| self.in_bounds(p))
    }

    /// Every position, row-major (the order of [`Grid::cells`]).
    pub fn positions(&self) -> impl Iterator<Item = Pos> + use<T> {
        let (w, h) = (i32::from(self.width), i32::from(self.height));
        (0..h).flat_map(move |y| (0..w).map(move |x| Pos::new(x, y)))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn grid(w: u16, h: u16) -> Grid<u32> {
        let n = u32::from(w) * u32::from(h);
        Grid::from_cells(w, h, (0..n).collect()).unwrap_or_else(|| unreachable!())
    }

    #[test]
    fn manhattan_distance() {
        assert_eq!(Pos::manhattan(Pos::new(0, 0), Pos::new(3, 4)), 7);
        assert_eq!(Pos::manhattan(Pos::new(3, 4), Pos::new(0, 0)), 7);
        assert_eq!(Pos::manhattan(Pos::new(-2, 5), Pos::new(1, 1)), 7);
        assert_eq!(Pos::manhattan(Pos::new(2, 2), Pos::new(2, 2)), 0);
    }

    #[test]
    fn step_follows_dir_delta() {
        let p = Pos::new(5, 5);
        assert_eq!(p.step(Dir::Right), Pos::new(6, 5));
        assert_eq!(p.step(Dir::Down), Pos::new(5, 6));
        assert_eq!(p.step(Dir::Left), Pos::new(4, 5));
        assert_eq!(p.step(Dir::Up), Pos::new(5, 4));
    }

    #[test]
    fn from_cells_checks_count() {
        assert!(Grid::from_cells(2, 3, vec![0; 6]).is_some());
        assert!(Grid::from_cells(2, 3, vec![0; 5]).is_none());
        assert!(Grid::from_cells(2, 3, vec![0; 7]).is_none());
        let g = Grid::from_cells(0, 0, Vec::<u8>::new());
        assert!(g.is_some_and(|g| !g.in_bounds(Pos::new(0, 0))));
    }

    #[test]
    fn dimensions_and_cells() {
        let g = grid(3, 2);
        assert_eq!((g.width(), g.height()), (3, 2));
        assert_eq!(g.cells(), &[0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn bounds_and_get() {
        let g = grid(3, 2);
        assert!(g.in_bounds(Pos::new(0, 0)));
        assert!(g.in_bounds(Pos::new(2, 1)));
        assert!(!g.in_bounds(Pos::new(3, 0)));
        assert!(!g.in_bounds(Pos::new(0, 2)));
        assert!(!g.in_bounds(Pos::new(-1, 0)));
        assert!(!g.in_bounds(Pos::new(0, -1)));
        assert_eq!(g.get(Pos::new(0, 0)), Some(&0));
        assert_eq!(g.get(Pos::new(2, 0)), Some(&2));
        assert_eq!(g.get(Pos::new(1, 1)), Some(&4));
        assert_eq!(g.get(Pos::new(2, 1)), Some(&5));
        assert_eq!(g.get(Pos::new(3, 0)), None);
        assert_eq!(g.get(Pos::new(-1, 1)), None);
    }

    #[test]
    fn filled_and_get_mut() {
        let mut g = Grid::filled(3, 2, 7u8);
        assert_eq!((g.width(), g.height()), (3, 2));
        assert_eq!(g.cells(), &[7; 6]);
        if let Some(c) = g.get_mut(Pos::new(2, 1)) {
            *c = 9;
        }
        if let Some(c) = g.get_mut(Pos::new(1, 0)) {
            *c = 5;
        }
        assert_eq!(g.cells(), &[7, 5, 7, 7, 7, 9]);
        assert!(g.get_mut(Pos::new(3, 0)).is_none());
        assert!(g.get_mut(Pos::new(0, -1)).is_none());
    }

    #[test]
    fn neighbors_in_fixed_order() {
        let g = grid(3, 3);
        let n: Vec<Pos> = g.neighbors4(Pos::new(1, 1)).collect();
        assert_eq!(
            n,
            [
                Pos::new(2, 1),
                Pos::new(1, 2),
                Pos::new(0, 1),
                Pos::new(1, 0)
            ]
        );
        let corner: Vec<Pos> = g.neighbors4(Pos::new(0, 0)).collect();
        assert_eq!(corner, [Pos::new(1, 0), Pos::new(0, 1)]);
        let far: Vec<Pos> = g.neighbors4(Pos::new(2, 2)).collect();
        assert_eq!(far, [Pos::new(1, 2), Pos::new(2, 1)]);
    }

    #[test]
    fn positions_are_row_major() {
        let g = grid(2, 2);
        let p: Vec<Pos> = g.positions().collect();
        assert_eq!(
            p,
            [
                Pos::new(0, 0),
                Pos::new(1, 0),
                Pos::new(0, 1),
                Pos::new(1, 1)
            ]
        );
        for (pos, cell) in g.positions().zip(g.cells()) {
            assert_eq!(g.get(pos), Some(cell));
        }
    }

    proptest! {
        #[test]
        fn neighbors_stay_in_bounds(w in 0u16..20, h in 0u16..20, x in -3i32..25, y in -3i32..25) {
            let g = grid(w, h);
            let pos = Pos::new(x, y);
            for n in g.neighbors4(pos) {
                prop_assert!(g.in_bounds(n));
                prop_assert_eq!(Pos::manhattan(pos, n), 1);
            }
            if g.in_bounds(pos) {
                let expected = Dir::ALL.iter().filter(|&&d| g.in_bounds(pos.step(d))).count();
                prop_assert_eq!(g.neighbors4(pos).count(), expected);
            }
        }
    }
}
