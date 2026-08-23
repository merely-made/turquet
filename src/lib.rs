/*
Copyright (c) 2015, 2016 Saurav Sachidanand

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate hifitime;
extern crate sofars;

#[cfg(feature = "verify")]
extern crate anise;

#[macro_use]
mod util;

#[macro_use]
mod coords;
mod aberr;
mod angle;
pub mod apparent;
mod asteroid;
mod atmos;
mod binary_star;
mod consts;
mod ecliptic;
pub mod foundation;
mod interpol;
mod lunar;
mod misc;
mod nutation;
mod orbit;
pub mod orientation;
mod parallax;
mod planet;
mod pluto;
mod precess;
mod star;
mod sun;
mod time;
mod transit;

/// The inherited astro-rust catalogue. Its anonymous scalar contracts are
/// retained for migration and are not Turquet's primary API.
pub mod compat;
#[cfg(feature = "verify")]
pub mod verify;
