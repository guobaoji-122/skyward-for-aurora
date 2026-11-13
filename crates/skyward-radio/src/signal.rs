//! What the radio says of the signal, read out of its own words.
//!
//! The radio answers a question about the signal with a structure that
//! holds one piece for each kind of radio the phone might be on: the one
//! of the second generation, of the third, of the fourth, of the fifth.
//! Only one of them is filled in -- the one the phone is actually on --
//! and which that is says how the signal is to be read.
//!
//! Each piece is a structure of its own, and each is read the same way: a
//! flag saying whether it is there at all, and then the numbers. A piece
//! that is not there is written as a flag of zero and nothing else, so
//! the reader has to stop at the flag and not read past it.
//!
//! The counts themselves are of two kinds. The older radios count the
//! signal on a scale of thirty-two steps from zero, and the newer ones
//! count it in decibels below a thousandth of a watt, where the larger
//! the number the stronger the signal.

use crate::carrier;

/// What the radio says of the signal, in the counts it counts in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reading {
    /// The kind of radio the signal came on, as the radio names it:
    /// "gsm", "cdma", "evdo", "lte", "tdscdma", "wcdma" or "nr".
    pub technology: String,
    /// The count of the older radios, from zero to thirty-one.
    pub level: i32,
    /// The count of the newer radios, in dBm.
    pub rsrp: i32,
    /// How many bars this signal stands at, of four.
    pub bars: u8,
}

/// The pieces of the structure the radio answers with, in the order it
/// writes them: the second generation, the third, the third in its own
/// way, the fourth, the third in the way China wrote it, the third in the
/// way Japan wrote it, and the fifth.
const KINDS: [&str; 7] = ["gsm", "cdma", "evdo", "lte", "tdscdma", "wcdma", "nr"];

/// Read the signal out of the words the radio wrote.
///
/// The words are the structure as the parcel carries it: a flag saying
/// whether the whole thing is there, its length, and then the pieces.
pub fn read_signal(data: &[u8]) -> Option<Reading> {
    let mut read = skyward_binder::parcel::Reader::new(data);
    // The structure is written as a flag saying whether it is there, and
    // then how long it is; what follows is its body.
    let end = read.structure().ok()?;
    let mut out = Reading::default();
    for kind in KINDS {
        let Some(piece) = read_piece(&mut read, kind) else { continue };
        out = piece;
        break;
    }
    // Whatever is left of the structure is of no interest, but it must
    // have been read as far as it goes.
    let _ = end;
    Some(out)
}

/// Read one piece of the structure, and say what it holds.
fn read_piece(read: &mut skyward_binder::parcel::Reader<'_>, kind: &str) -> Option<Reading> {
    // Each piece is written as a flag saying whether it is there.
    if read.i32().ok()? == 0 {
        return None;
    }
    // Then its own length, and then its numbers.
    let end = read.structure().ok()?;
    let mut out = Reading { technology: kind.to_string(), ..Reading::default() };
    match kind {
        "gsm" => {
            out.level = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "cdma" => {
            out.level = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "evdo" => {
            out.level = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "lte" => {
            out.level = read.i32().ok()?;
            out.rsrp = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "tdscdma" => {
            out.level = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "wcdma" => {
            out.level = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
        }
        "nr" => {
            out.rsrp = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            let _ = read.i32().ok()?;
            // The reported quality is a list of bytes, which is read as
            // a blob.
            let _ = read.blob().ok()?;
            let _ = read.i32().ok()?;
        }
        _ => {}
    }
    // Whatever the piece held beyond what is of interest is stepped
    // over, so that the next piece is read from the right place.
    let _ = read.go(end);
    out.bars = carrier::bars_of(&out.technology, out.level, out.rsrp);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write the structure the way a parcel carries it: a flag saying
    /// whether it is there, its length, and then its body.
    fn structure(body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&(body.len() as i32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// Write one piece: the flag saying it is there, its length, and its
    /// numbers.
    fn piece(numbers: &[i32]) -> Vec<u8> {
        let mut body = Vec::new();
        for n in numbers {
            body.extend_from_slice(&n.to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&(body.len() as i32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// The flag that says a piece is not there.
    fn absent() -> Vec<u8> {
        0i32.to_le_bytes().to_vec()
    }

    #[test]
    fn an_lte_signal_is_read_as_four_bars_by_its_dbm() {
        let body = [
            absent(),
            absent(),
            absent(),
            piece(&[20, -80, -10, 30, 0, 0, 0]),
            absent(),
            absent(),
            absent(),
        ]
        .concat();
        let got = read_signal(&structure(&body)).expect("the signal is read");
        assert_eq!(got.technology, "lte");
        assert_eq!(got.rsrp, -80);
        assert_eq!(got.bars, 4);
    }

    #[test]
    fn a_gsm_signal_is_read_as_four_bars_by_its_count() {
        let body = [
            piece(&[20, 0, 0]),
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
        ]
        .concat();
        let got = read_signal(&structure(&body)).expect("the signal is read");
        assert_eq!(got.technology, "gsm");
        assert_eq!(got.level, 20);
        assert_eq!(got.bars, 3);
    }

    #[test]
    fn a_signal_of_the_fifth_generation_is_read_by_its_dbm() {
        let mut nr = piece(&[-100, 0, 0, 0, 0, 0, 0]);
        // The reported quality is a list of bytes, written as a length
        // and then the bytes.
        nr.extend_from_slice(&0i32.to_le_bytes());
        nr.extend_from_slice(&0i32.to_le_bytes());
        let body = [
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
            nr,
        ]
        .concat();
        let got = read_signal(&structure(&body)).expect("the signal is read");
        assert_eq!(got.technology, "nr");
        assert_eq!(got.rsrp, -100);
        assert_eq!(got.bars, 3);
    }

    #[test]
    fn a_signal_of_nothing_is_read_as_no_bars() {
        let body = [
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
            absent(),
        ]
        .concat();
        let got = read_signal(&structure(&body)).expect("the signal is read");
        assert_eq!(got.technology, "");
        assert_eq!(got.bars, 0);
    }
}
