//! On-board user LEDs

use hal::gpio::gpio::PIN;
use hal::gpio::gpio::{
    PIN10, PIN11, PIN12, PIN13, PIN14, PIN15, PIN4, PIN5, PIN6, PIN7, PIN8, PIN9,
};
use hal::gpio::{Output, PushPull};
use hal::prelude::*;

type LED = PIN<Output<PushPull>>;
type Image = [[u8; 5]; 5];
type DisplayBuffer = [[u8; 9]; 3];

const LED_LAYOUT: [[(usize, usize); 5]; 5] = [
    [(0, 0), (1, 3), (0, 1), (1, 4), (0, 2)],
    [(2, 3), (2, 4), (2, 5), (2, 6), (2, 7)],
    [(1, 1), (0, 8), (1, 2), (2, 8), (1, 0)],
    [(0, 7), (0, 6), (0, 5), (0, 4), (0, 3)],
    [(2, 2), (1, 6), (2, 0), (1, 5), (2, 1)],
];

/// Array of all the LEDs in the 5x5 display on the board
pub struct LedDisplay {
    rows: [LED; 3],
    cols: [LED; 9],
    row: usize,
    buffer: DisplayBuffer,
    next_buffer: DisplayBuffer,
    next_updated: bool,
}

impl LedDisplay {
    /// Initializes all the user LEDs
    pub fn new(
        col1: PIN4<Output<PushPull>>,
        col2: PIN5<Output<PushPull>>,
        col3: PIN6<Output<PushPull>>,
        col4: PIN7<Output<PushPull>>,
        col5: PIN8<Output<PushPull>>,
        col6: PIN9<Output<PushPull>>,
        col7: PIN10<Output<PushPull>>,
        col8: PIN11<Output<PushPull>>,
        col9: PIN12<Output<PushPull>>,
        row1: PIN13<Output<PushPull>>,
        row2: PIN14<Output<PushPull>>,
        row3: PIN15<Output<PushPull>>,
    ) -> Self {
        let mut retval = LedDisplay {
            rows: [row1.downgrade(), row2.downgrade(), row3.downgrade()],
            cols: [
                col1.downgrade(),
                col2.downgrade(),
                col3.downgrade(),
                col4.downgrade(),
                col5.downgrade(),
                col6.downgrade(),
                col7.downgrade(),
                col8.downgrade(),
                col9.downgrade(),
            ],
            buffer: [[0; 9]; 3],
            next_buffer: [[0; 9]; 3],
            next_updated: false,
            row: 0,
        };
        // This is needed to reduce flickering on reset
        retval.clear();
        retval
    }

    /// Clear display
    pub fn clear(&mut self) {
        for row in &mut self.rows {
            row.set_low();
        }
        for col in &mut self.cols {
            col.set_high();
        }
    }

    /// Convert 5x5 display image to 3x9 matrix image
    fn update_next(&mut self, image: Image) {
        for (led_display_row, layout_row) in image.iter().zip(LED_LAYOUT.iter()) {
            for (led_display_val, layout_loc) in led_display_row.iter().zip(layout_row) {
                self.next_buffer[layout_loc.0][layout_loc.1] = *led_display_val;
            }
        }
        self.next_updated = true;
    }

    /// Display 5x5 display image
    pub fn display(&mut self, image: Image) {
        self.update_next(image);
    }

    pub fn update_row(&mut self) {
        for col_sig in self.cols.iter_mut() {
            col_sig.set_high();
        }
        // last row
        {
            let row_sig = self.rows.get_mut(self.row).unwrap();
            row_sig.set_low();
        }
        // update row
        self.row = (self.row + 1) % self.rows.len();
        // update buffer
        if self.row == 0 && self.next_updated {
            for (dst_row, src_row) in self.buffer.iter_mut().zip(self.next_buffer.iter()) {
                for (dst, src) in dst_row.iter_mut().zip(src_row.iter()) {
                    *dst = *src;
                }
            }
            self.next_updated = false;
        }
        // new row
        let row_sig = self.rows.get_mut(self.row).unwrap();
        row_sig.set_high();
        let row_vals = self.buffer[self.row];
        for (col_sig, col_val) in self.cols.iter_mut().zip(row_vals.iter()) {
            if *col_val > 0 {
                col_sig.set_low();
            }
            else {
                col_sig.set_high();
            }
        }
    }
}