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
        let screen_layout = self.create_layout_for_sway();
        for screen in screen_layout {
            let string = format!("output {} pos {} {}", screen.name, screen.x, screen.y);
            println!("Running: {string}");
            connection.run_command(string).unwrap();
        }
    }

    fn create_layout_for_sway(&self) -> Vec<Screen> {
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
        screen_layout
    }

    pub(crate) fn print(&self) {
        let divider = "==============".repeat(self.inner[0].len());
        println!("{divider}");
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
            println!("\n{divider}");
        }
    }

    pub(crate) fn shrink_padding(&mut self) {
        let has_padding_at_top = self.inner.first().unwrap().iter().all(|x| x.is_none());
        if has_padding_at_top {
            self.inner.remove(0);
        }

        let has_padding_at_bottom = self.inner.last().unwrap().iter().all(|x| x.is_none());
        if has_padding_at_bottom {
            self.inner.pop();
        }

        let mut has_padding_left = true;
        let mut has_padding_right = true;
        for row in &self.inner {
            if row.first().unwrap().is_some() {
                has_padding_left = false;
            }
            if row.last().unwrap().is_some() {
                has_padding_right = false;
            }
        }

        if has_padding_left {
            for row in &mut self.inner {
                row.remove(0);
            }
        }

        if has_padding_right {
            for row in &mut self.inner {
                row.pop();
            }
        }
    }

    pub(crate) fn grow_padding(&mut self) {
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
        self.inner[0].len()
    }

    #[cfg(test)]
    pub(crate) fn height(&self) -> usize {
        self.inner.len()
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

    pub(crate) fn move_screen(&mut self, src_row_idx: usize, src_col_idx: usize, dst_row_idx: usize, dst_col_idx: usize) {
        println!("Moving from: ({}, {}) => ({}, {})", src_row_idx, src_col_idx, dst_row_idx, dst_col_idx);
        self.inner[dst_row_idx][dst_col_idx] = std::mem::replace(&mut self.inner[src_row_idx][src_col_idx], None);
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

    #[test]
    fn works_with_one_screen() {
        let screens = vec![Screen {
            x: 0,
            y: 0,
            width: 1920,
            height: 1200,
            name: "eDP-1".to_owned(),
        }];

        let mut grid = ScreenGrid::from_screens(screens);

        assert_eq!(1, grid.height());
        assert_eq!(1, grid.width());

        grid.grow_padding();

        assert_eq!(3, grid.height());
        assert_eq!(3, grid.width());
    }

    #[test]
    fn works_with_two_screens() {
        let screens = vec![
            Screen {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1200,
                name: "eDP-1".to_owned(),
            }, Screen {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                name: "HDMI-A-1".to_owned(),
            },
        ];

        let mut grid = ScreenGrid::from_screens(screens);
        assert_eq!(1, grid.height());
        assert_eq!(2, grid.width());

        grid.grow_padding();

        assert_eq!(3, grid.height());
        assert_eq!(4, grid.width());
    }

    #[test]
    fn three_differently_sized_screens() {
        // DP-8  : 0,0    : 1920x1200
        // DP-7  : 1920,0 : 2560x1440
        // eDP-1 : 4480,0 : 2560x1600
        let screens = vec![
            Screen {
                x: 4480,
                y: 0,
                width: 2560,
                height: 1600,
                name: "eDP-1".to_string(),
            },
            Screen {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
                name: "DP-7".to_string(),
            },
            Screen {
                x: 0,
                y: 0,
                width: 1920,
                height: 1200,
                name: "DP-8".to_string(),
            },
        ];

        let mut grid = ScreenGrid::from_screens(screens);

        assert_eq!(1, grid.height());
        assert_eq!(3, grid.width());

        grid.grow_padding();

        assert_eq!(3, grid.height());
        assert_eq!(5, grid.width());

        grid.move_screen(1, 3, 0, 2);

        grid.shrink_padding();
        grid.print();
        grid.shrink_padding();
        grid.print();

        grid.grow_padding();
        grid.print();

        assert_eq!(4, grid.height());
        assert_eq!(4, grid.width());

        let layout = grid.create_layout_for_sway();

        for screen in &layout {
            println!("{screen:?}");
        }

        let mut grid = ScreenGrid::from_screens(layout);

        grid.print();
    }


    #[test]
    fn parse_three_screens() {
        // DP-8  : 0,0    : 1920x1200
        // DP-7  : 1920,0 : 2560x1440
        // eDP-1 : 4480,0 : 2560x1600
        let screens = vec![
            Screen {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1600,
                name: "eDP-1".to_string(),
            },
            Screen {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
                name: "DP-7".to_string(),
            },
            Screen {
                x: 1920,
                y: 1600,
                width: 1920,
                height: 1200,
                name: "DP-8".to_string(),
            },
        ];

        let mut grid = ScreenGrid::from_screens(screens);

        assert_eq!(1, grid.height());
        assert_eq!(3, grid.width());
    }

    }