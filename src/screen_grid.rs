use swayipc::{Connection, Output};

#[derive(Clone, Debug)]
pub struct Screen {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub name: String,
}

#[derive(Debug)]
pub(crate) struct ScreenGrid {
    pub(crate) inner: Vec<Vec<Option<Screen>>>,
}

impl ScreenGrid {
    pub(crate) fn from_outputs(outputs: Vec<Output>) -> ScreenGrid {
        Self::from_screens(outputs
            .iter()
            .map(|output| Screen {
                x: output.rect.x,
                y: output.rect.y,
                width: output.rect.width,
                height: output.rect.height,
                name: output.name.clone(),
            })
            .collect()
        )
    }

    pub(crate) fn from_screens(screens: Vec<Screen>) -> ScreenGrid {
        let screens_by_col = Self::split_by(
            screens.clone(),
            |screen| screen.x,
            |screen| screen.y,
        );
        let screens_by_row = Self::split_by(
            screens.clone(),
            |screen| screen.y,
            |screen| screen.x,
        );

        println!("By col:");
        for c in &screens_by_col {
            println!("New col!");
            for a in c {
                println!("{}: {},{} : {}x{}", a.name, a.x, a.y, a.width, a.height);
            }
        }

        println!("By row:");
        for c in &screens_by_row {
            println!("New row!");
            for a in c {
                println!("{}: {},{} : {}x{}", a.name, a.x, a.y, a.width, a.height);
            }
        }

        let mut grid: Vec<Vec<Option<Screen>>> = vec![vec![None; screens_by_col.len()]; screens_by_row.len()];
        println!("{}x{}", grid.len(), grid[0].len());

        for (row_idx, row) in screens_by_row.iter().enumerate() {
            for screen_in_row in row {
                for (col_idx, col) in screens_by_col.iter().enumerate() {
                    if contains(screen_in_row, col) {
                        grid[row_idx][col_idx] = Some((*screen_in_row).clone());
                    }
                }
            }
        }

        ScreenGrid {
            inner: grid,
        }
    }

    pub(crate) fn apply_changes(&self, connection: &mut Connection) {
        let mut screen_layout = vec![];

        for (row_idx, row) in self.inner.iter().enumerate() {
            for (col_idx, screen) in row.iter().enumerate() {
                if let Some(screen) = screen {
                    let (x, y) = self.calculate_coordinates(row_idx, col_idx);
                    screen_layout.push(Screen {
                        x,
                        y,
                        ..screen.clone()
                    });
                }
            }
        }
        for screen in screen_layout {
            let string = format!("output {} pos {} {}", screen.name, screen.x, screen.y);
            println!("Running: {string}");
            connection.run_command(string).unwrap();
        }
    }

    pub(crate) fn print(&self) {
        println!("============================");
        for row in &self.inner {
            for col in row {
                print!("== ");
                if let Some(c) = col {
                    print!("{: >8}", c.name);
                } else {
                    print!("{: >8}", "None");
                }
                print!(" ==");
            }
            println!("\n============================");
        }
    }

    pub(crate) fn recalculate_padding(&mut self) {
        if needs_space_in_row(&self.inner.first().unwrap()) {
            // add space at the top
            self.inner.insert(0, vec![None; self.inner[0].len()]);
        }

        let mut needs_space_left = false;
        let mut needs_space_right = false;
        for row in &self.inner {
            if row.first().unwrap().is_some() {
                needs_space_left = true;
            }
            if row.last().unwrap().is_some() {
                needs_space_right = true;
            }
        }

        if needs_space_left {
            // add space on the left
            for row in &mut self.inner {
                row.insert(0, None);
            }
        }

        if needs_space_right {
            // add space on the right
            for row in &mut self.inner {
                row.push(None);
            }
        }

        if needs_space_in_row(&self.inner.last().unwrap()) {
            // add space at the bottom
            self.inner.push(vec![None; self.inner[0].len()]);
        }
    }

    pub(crate) fn split_by(mut screens: Vec<Screen>, sort_outer: fn(&Screen) -> i32, sort_inner: fn(&Screen) -> i32) -> Vec<Vec<Screen>> {
        screens.sort_by_key(sort_outer);
        let last_value = 0;
        let mut columned_outputs = vec![vec![]];
        for output in screens {
            if sort_outer(&output) == last_value {
                columned_outputs.last_mut().unwrap().push(output);
            } else {
                columned_outputs.push(vec![output]);
            }
        }

        for column in &mut columned_outputs {
            column.sort_by_key(sort_inner);
        }
        columned_outputs
    }

    #[cfg(test)]
    pub(crate) fn width(&self) -> usize {
        self.inner.len()
    }

    #[cfg(test)]
    pub(crate) fn height(&self) -> usize {
        self.inner[0].len()
    }

    fn calculate_coordinates(&self, row_idx: usize, col_idx: usize) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;
        for row_idx in 0..row_idx {
            x += self.inner[row_idx].iter()
                .filter_map(|s| s.as_ref())
                .map(|s| s.height)
                .min().unwrap_or(0)
        }
        for col_idx in 0..col_idx {
            y += self.inner.iter()
                .filter_map(|s| s.get(col_idx))
                .filter_map(|s| s.as_ref())
                .map(|s| s.width)
                .min().unwrap_or(0)
        }

        (y, x)
    }
}

fn contains(screen_in_row: &Screen, col: &Vec<Screen>) -> bool {
    for c in col {
        if c.name == screen_in_row.name {
            return true;
        }
    }
    return false;
}

fn needs_space_in_row(p0: &Vec<Option<Screen>>) -> bool {
    p0.iter().any(|a| a.is_some())
}

#[cfg(test)]
mod test {
    use crate::screen_grid::ScreenGrid;
    use super::Screen;

    const MAIN_SCREEN: Screen = Screen {
        x: 0,
        y: 0,
        width: 1920,
        height: 1200,
        name: String::new(),
    };

    #[test]
    fn works_with_one_screen() {
        let mut main_screen = Screen::from(MAIN_SCREEN);
        main_screen.name = "eDP-1".to_owned();

        let mut grid = ScreenGrid::from_screens(vec![MAIN_SCREEN]);

        assert_eq!(1, grid.height());
        assert_eq!(1, grid.width());

        grid.recalculate_padding();

        assert_eq!(3, grid.height());
        assert_eq!(3, grid.width());
    }
}