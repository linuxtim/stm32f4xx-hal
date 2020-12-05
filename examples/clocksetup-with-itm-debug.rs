#![no_std]
#![no_main]


use cortex_m::iprintln;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
//use panic_halt as _;

use stm32f4xx_hal::{prelude::*, stm32, time::MegaHertz};

/// Compile this example with e.g.  `cargo build --example clocksetup-with-itm-debug --features="rt
/// stm32f411"`
///
/// This example sets the core clock to 16MHz, and then outputs some data using the
/// "Instrumentation Trace Macrocell" (ITM) using ITM "stimulus port 0".
///
/// Before ITM is setup, this code uses its `dbgln!()` macro to print debug data via other means.
///
/// ITM is an ARM specification, and is an optional core peripheral on Cortex M4 (and most other)
/// ARM microcontroller cores.
///
/// It allows debug information to be reported to an external device with very low latency and
/// overhead.  For this reason, it can be useful when debugging, or profiling timing-sensitive
/// code.
///
/// ITM is included in all STM32F4xx microcontrollers.
///
/// ITM can be exported with JTAG on some stm32f4 (the more common lower pin count stm32f4 do not
/// have this feature).
///
/// ITM data is more usually output from STM32F4xx using the SWO pin.
///
/// The SWO ITM output can be setup in two different ways:
///
/// 1. Code running on the host can use a debug probe to directly set the necessary registers on
///    the target.
///
/// 2. The target code itself can setup the registers to enable the SWO data itself (this has the
///    advantage of not requiring a debug probe at all - the SWO data can be read with just a UART
///    periperhal on the host.
///
/// In either case, the SWO output signal is configured by dividing the core clock rate by a 16 bit
/// integer.
///
/// For this reason, the core clock speed must be known in order to configure the SWO baud rate.
///
/// Because SWO in an asychronous signal, the debug probe needs to know what speed to expect to
/// receive data at on the SWO pin.
///
/// The SWO pin can be set to operate in two different modes:
///
/// 1. Manchester Encoded bitstream.
///
/// 2. UART compatible.
///
/// The Manchester encoded scheme supports higher data rates, and is more tollerant of mismatched
/// send/receive baud rates (± 10%).
///
/// UART (NRZ) mode has the advantage of widespread support, the sending and recieving baud rates
/// should match to ±5%.
///
/// The SWO signal is present on the `PB3` pin of stm32f4xx MCUs (configuring `PB3` for other
/// purposes will disable SWO output).
///
/// The baud rate is configured using the Trace Port Interface Unit Asynchronous Clock Prescaler
/// Register (TPIU_ACPR), which is described in the "ARMv7-M Architecture Reference Manual".
///
/// n.b. Code which changes the core clock will also change the rate of the SWO output.  A commonly
/// encountered example would be WFI (Wait For Interrupt), which will cause the processor clock to
/// stop, and the ITM receiver to lose synchronisation.
///
/// n.b. In common with UART communication, inaccuracies in the internal (on-board) oscilator can
/// cause the baud rate to be set inaccurately.  This can occasionally be a problem, and should be
/// borne in mind for that reason (e.g. when running the MCU at low voltages, and extremes of
/// temperature etc.).
///
/// ST DocID026289 Rev 7 specifies the STM32F411xC/E HSI accuracy to be ±1% at 3.3v, 25°C.  The
/// worst-case accuracy is +5.5% / -8% over the range of -40°C to 125°C.
///
/// TODO:
///
/// - Add standalone example where we configure TPIU for debugging without a probe.
/// - Use a feature flag for standalone (no debug probe, just UART) config?
///
/// Let's be fairly conservative in our choice of core clock speed.  If the user specifies an
/// incorrect HSE value e.g. 8 MHz, when there's really a 25 MHz part attached, then the core will
/// run at 32 MHz - and it should still work (at least it did when I tested it), so they stands a
/// better chance of working out what's wrong (calculated ITM baud will of course be incorrect)...


macro_rules! dbgln {
    ($fmt:expr) => { hprintln!($fmt); };
    ($fmt:expr, $($arg:tt)*) => { hprintln!($fmt, $($arg)*); };
}

#[entry]
fn main() -> ! {
    let desired_core_freq = 16.mhz();
    let dp = stm32::Peripherals::take().unwrap();
    let mut cp = cortex_m::peripheral::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();


    dbgln!("Hello via semihosting..."); 
    dbgln!("Semihosting is relatively SLOW, and will only run with a debug probe attached and configured.");

    dbgln!( "Let's try and set the core clock to {:?}...", desired_core_freq) .ok();

    #[cfg(not(any(
        feature = "clocks_example_ext_osc8",
        feature = "clocks_example_ext_osc25",
    )))]
    // All stm32f4xx have an internal 16 MHz (ish) RC oscillator.  This is usually fine for ITM SWO
    // output (see caveat above), so we default to using this.
    
    // Use the STM32F4xx-specific PLLs to configure the desired core clock based on the RC
    // oscillator's nominal frequency.
    let clocks = rcc.cfgr.sysclk(desired_core_freq).freeze();

    // Alternaively, if one is connected to the MCU, we can use a higher accuracy external
    // oscillator ("HSE") as the clock reference, but the frequency of the external osciallator
    // speed is then up to the board designer, so this is more error prone.

    #[cfg(feature = "clocks_example_ext_osc8")]
    // 8 MHz external oscillator e.g. ST NUCLEO-F411RE in default factory config - see ST document:
    // UM1724 - https://www.st.com/resource/en/user_manual/dm00105823.pdf section 6.7.1
    let HSE_FREQ: MegaHertz = 8.mhz();

    #[cfg(feature = "clocks_example_ext_osc25")]
    // 25 MHz external oscillator - e.g. WeAct Studio STM32F4x1 MiniF4:
    // https://github.com/WeActTC/MiniF4-STM32F4x1
    const HSE_FREQ: MegaHertz = 25.mhz();

    #[cfg(any(
        feature = "clocks_example_ext_osc8",
        feature = "clocks_example_ext_osc25",
    ))]
    {
        dbgln!(
            "We are about to configure the MCU clock hardware to expect a high speed external (HSE) oscillator of speed: {:?}",
            HSE_FREQ).ok();
        dbgln!(
            "If there's IS an HSE attached, and it's a lot faster than {:?}, then the MCU might crash, or even be physically damaged.",
            HSE_FREQ).ok();
        dbgln!("If there's no HSE attached or the HSE fails to start, then the MCU will STOP, and you will hear no more from me...");
        dbgln!("(...until you power cycle, or reset the board, that is...)");
    }

    #[cfg(any(
        feature = "clocks_example_ext_osc8",
        feature = "clocks_example_ext_osc25",
    ))]
    let clocks = rcc
        .cfgr
        .use_hse(HSE_FREQ)
        .sysclk(DESIRED_CORE_FREQ)
        .freeze();

    // Trace Port Interface Unit - Asynchronous Clock Prescaler Register
    // Ref "ARMv7-M Architecture Reference Manual" section C1.10.4

    let acpr: u32 = cp.TPIU.acpr.read();

    dbgln!("Asynchronous Clock Prescaler Register value: {}", acpr);
    dbgln!(
        "SWO baud rate == (core clock / tpiu.acpr) == {}",
        clocks.sysclk().0 / acpr
    );

    // Trace Port Interface Unit
    // Selected Pin Protocol Register
    let sppr: u32 = cp.TPIU.sppr.read();

    match sppr & 0b11 {
        0b00 => dbgln!("TPIU output mode is parallel (e.g. JTAG)"),
        0b01 => dbgln!("TPIU output mode is serial (SWO) - Manchester Encoding"),
        0b10 => dbgln!("TPIU output mode is serial (SWO) - NRZ/UART Encoding"),
        _ => dbgln!("TPIU Undefined (reserved) value!!"),
    };

    dbgln!(
        "SWO baud rate == (core clock / tpiu.acpr) == {}",
        clocks.sysclk().0 / acpr
    );

    //let bool jtag_enabled = acpr.

    // We'll use ITM "stimulus port" 0 for our debug output.
    let stim = &mut cp.ITM.stim[0];

    iprintln!(stim, "Hello, ITM!");
    iprintln!(stim, "Sometimes the best thing to do is to panic!().");
    iprintln!(stim, "This is one of those times...");
    panic!();
    loop {
        cortex_m::asm::nop();
    }
}
