#![no_main]
#![no_std]

use core::arch::asm;
use rtt_target::{rtt_init_print, rprintln};
use cortex_m_rt::{entry, exception};
use embedded_graphics::fonts::Font12x16;
use embedded_graphics::image::ImageBmp;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rectangle};
use embedded_hal::digital::v2::OutputPin;
// use generic_array::{ArrayLength, GenericArray};
//use ili9341;
use oorandom;
use panic_halt as _;
use core::mem::MaybeUninit;

use crate::hal::{
    prelude::*,
    serial::{self, Serial},
    spi::Spi,
    stm32
};
use stm32f4xx_hal as hal;

mod ili9341_controller;
use crate::ili9341_controller:: {spi, Ili9341, Orientation};

mod scheduler;
use crate::scheduler::{Scheduler, SchedulingStrategy};
use crate::scheduler::context_switch::{Task, TaskHandler};

// Dummy Werte, werden in init_scheduler sinnvoller initialisiert
static mut SCHEDULER: Scheduler = Scheduler {
    strategy: SchedulingStrategy::RoundRobin,
    task_queue: MaybeUninit::uninit(),
    num_tasks: 0,
    current_task_id: 0,
    next_task: 0 as *mut u32,
    current_task: 0 as *mut u32
};

const TASK_STACK_SIZE: u32 = 40;  // in 4 Byte also 160 Byte

unsafe fn init_scheduler(strategy: SchedulingStrategy) {
    SCHEDULER = Scheduler::new(strategy);
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    if let (Some(p), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        let _start = cortex_m_rt::heap_start() as usize;
        let _size = 1024;

        let rcc = p.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(180.mhz()).freeze();

        let gpiog = p.GPIOG.split();
        let gpioa = p.GPIOA.split();
        let gpiof = p.GPIOF.split();
        let gpioc = p.GPIOC.split();
        let gpiod = p.GPIOD.split();
        let gpiob = p.GPIOB.split();

        let mut led0 = gpiog.pg13.into_push_pull_output();
        let mut led1 = gpiog.pg14.into_push_pull_output();

        let mut delay = hal::delay::Delay::new(cp.SYST, clocks);

        let en = gpiof.pf10.into_push_pull_output();
        // ------------ SPI INTERFACE SETUP ----------------

        let spi = Spi::spi5(
            p.SPI5,
            (
                gpiof.pf7.into_alternate_af5(),
                hal::spi::NoMiso,
                gpiof.pf9.into_alternate_af5(),
            ),
            spi::MODE,
            20_000_000.hz(),
            clocks,
        );

        let cs = gpioc.pc2.into_push_pull_output();
        let dc = gpiod.pd13.into_push_pull_output();

        let if_spi = spi::SpiInterface::new(spi, cs, dc);

        // ---------- SPI INTERFACE SETUP ENDE -------------

        let mut lcd = Ili9341::new(if_spi, en, &mut delay).unwrap();
        lcd.set_orientation(Orientation::Landscape)
            .unwrap();

        let temp_cal = hal::signature::VtempCal30::get().read();
        rprintln!("Temp Cal 30: {}", temp_cal);

        rprintln!("Initialised");

        let image = ImageBmp::new(include_bytes!("../cat.bmp"))
            .unwrap()
            .translate(Point::new(30, 30));
        lcd.draw(&image);

        let mut rng = oorandom::Rand32::new(0);
        let mut i = 0;
        let mut x: i32 = 10;
        let mut y: i32 = 10;
        let r = 20;
        let mut xvel = 4;
        let mut yvel = 4;
        loop {
            led0.set_high().unwrap();
            led1.set_high().unwrap();

            let col = Rgb565::new(
                rng.rand_range(0..255) as u8,
                rng.rand_range(0..255) as u8,
                rng.rand_range(0..255) as u8,
            );

            if x >= lcd.width() as i32 {
                xvel = -1 * rng.rand_range(1..10) as i32
            } else if x <= 0 {
                xvel = rng.rand_range(1..10) as i32;
            }
            if y >= lcd.height() as i32 {
                yvel = -1 * rng.rand_range(1..10) as i32;
            } else if y <= 0 {
                yvel = rng.rand_range(1..10) as i32;
            }

            x += xvel;
            y += yvel;

            let rect = Rectangle::new(Point::new(x, y), Point::new(x+r as i32, y+r as i32)).fill(Some(col));
            // let text = Font12x16::render_str("Hello world")
            //     .style(Style::stroke(Rgb565::RED))
            //     .translate(Point::new(x, y));
            // let c = Circle::new(Point::new(x, y), r)
            //     .stroke(Some(col))
            //     .stroke_width(5); //.fill(Some(Rgb565::BLUE));
            lcd.draw(rect);
            // lcd.draw(c);
            // lcd.draw(text);

            led0.set_low().unwrap();
            led1.set_low().unwrap();
            rprintln!("Loop: {}", i);
            i = i + 1;
        }
    }

    unsafe { init_scheduler(SchedulingStrategy::RoundRobin); }

    loop {}
}

#[exception]
fn PendSV() {
    unsafe {
    let local_curr_task = SCHEDULER.current_task;
    let local_next_task = SCHEDULER.next_task;
    
    /*
					// Inline Assembly für Context Switch
					// aus: https://youtu.be/TEq3-p0GWGI?si=2GW9c8E3DRtkuhEZ
    asm!(
    /* __disable_irq(); */
    "  CPSID I",

    /* if (local_curr_task != (OSThread *)0) { */
    "  LDR  r1,={0}",
    "  LDR  r1,[r1,#0x00]",
    "  CBZ  r1,2f ",

    /*     push registers r4-r11 on the stack */
    "  PUSH {{r4-r11}}",

    /*     local_curr_task->sp = sp; */
    "  LDR  r1,={0}",
    "  LDR  r1,[r1,#0x00]",
    "  STR  sp,[r1,#0x00]",
    /* } */

    "2:",
    /* sp = local_next_task->sp; */
    "  LDR  r1,={1}",
    "  LDR  r1,[r1,#0x00]",
    "  LDR  sp,[r1,#0x00]",

    /* local_curr_task = local_next_task; */
    "  LDR  r1,={1}",
    "  LDR  r1,[r1,#0x00]",
    "  LDR  r2,={0}",
    "  STR  r1,[r2,#0x00]",

    /* pop registers r4-r11 */
    "  POP  {{r4-r11}}",

    /* __enable_irq(); */
    "  CPSIE I",

    /* return to the next thread */
    "  BX  lr",
    in(reg) local_curr_task,
    in(reg) local_next_task,
    );
    */
    
    }
}