#![cfg_attr(not(test), no_std)]

use core::clone::Clone;
use core::cmp::Eq;
use core::cmp::PartialEq;
use core::derive;
use core::fmt::Debug;
use core::option::Option;
use core::option::Option::{None, Some};

pub const LIT_OPCODE: u8 = 0;
pub const DELTA_OPCODE: u8 = 1;
pub const RLE_OPCODE: u8 = 2;
pub const SEED_OPCODE: u8 = 3;
pub const DICT_OPCODE: u8 = 4;
pub const XOR_OPCODE: u8 = 5;
pub const RS_OPCODE: u8 = 6;
/// The `Opcode` enum represents the different operation codes that can be executed by the `execute` function.
///
/// Each variant corresponds to a specific operation.
///
/// Such as literal copy, delta encoding, run-length encoding, seeding, dictionary-based compression, XOR operations, and right shift operations.
///
/// The enum is represented as a `u8` for efficient storage and comparison.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opcode {
    /// Represents a literal copy operation.
    Lit = LIT_OPCODE,
    /// Represents a delta encoding operation.
    Delta = DELTA_OPCODE,
    /// Represents a run-length encoding operation.
    Rle = RLE_OPCODE,
    /// Represents a seeding operation.
    Seed = SEED_OPCODE,
    /// Represents a dictionary-based compression operation.
    Dict = DICT_OPCODE,
    /// Represents an XOR operation.
    Xor = XOR_OPCODE,
    /// Represents a right shift operation.
    Rs = RS_OPCODE,
}

impl Opcode {
    /// Converts a `u8` value to an `Opcode` variant. Returns `None` if the value does not correspond to any valid opcode.
    /// # Arguments
    /// * `value` - A `u8` value representing the opcode.
    /// # Example
    /// ```no_run
    /// use kpack::Opcode;
    /// use kpack::LIT_OPCODE;
    /// let opcode = Opcode::from_u8(LIT_OPCODE);
    /// assert_eq!(opcode, Some(Opcode::Lit));
    /// ```
    /// # Returns
    /// * `Option<Opcode>` - Returns `Some(Opcode)` if the value is valid, otherwise returns `None`.
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            LIT_OPCODE => Some(Self::Lit),
            DELTA_OPCODE => Some(Self::Delta),
            RLE_OPCODE => Some(Self::Rle),
            SEED_OPCODE => Some(Self::Seed),
            DICT_OPCODE => Some(Self::Dict),
            XOR_OPCODE => Some(Self::Xor),
            RS_OPCODE => Some(Self::Rs),
            _ => None,
        }
    }
}

/// Executes the `Lit` opcode, which copies a specified number of bytes from the payload to the output buffer.
///
/// # Arguments
/// * `param` - A parameter that may be used by certain opcodes.
/// * `payload` - The data associated with the operation.
/// * `output_buffer` - The buffer where the result of the operation will be stored.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 1024];
/// let payload = [1, 2, 3, 4];
/// execute(Opcode::Lit, 4, &payload, &mut output, None);
/// ```
///
pub fn execute_lit(param: u32, payload: &[u8], output_buffer: &mut [u8]) {
    let len = core::cmp::min(param as usize, output_buffer.len());
    let safe_len = core::cmp::min(len, payload.len());
    output_buffer[..safe_len].copy_from_slice(&payload[..safe_len]);
}

/// Executes the `Rle` opcode, which performs run-length encoding by repeating a specified byte value a given number of times in the output buffer.
/// # Arguments
/// * `param` - The number of times to repeat the byte value.
/// * `payload` - The data containing the byte value to be repeated.
/// * `output_buffer` - The buffer where the result of the RLE operation will be stored.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 10];
/// let payload = [b'X'];
/// execute(Opcode::Rle, 10, &payload, &mut output, None);
/// assert_eq!(&output, b"XXXXXXXXXX");
/// ```
pub fn execute_rle(param: u32, payload: &[u8], output_buffer: &mut [u8]) {
    let len = core::cmp::min(param as usize, output_buffer.len());
    if !payload.is_empty() {
        output_buffer[..len].fill(payload[0]);
    }
}

/// Executes the `Delta` opcode, which reconstructs data by applying delta instructions to a reference buffer.
/// # Arguments
/// * `payload` - The data containing the delta instructions.
/// * `output_buffer` - The buffer where the reconstructed data will be stored.
/// * `reference_buffer` - An optional buffer that serves as the reference for delta operations.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 1024];
/// let reference = [1, 2, 3, 4];
/// let payload = [0x01, 0x00, 0x00, 0x00, 0x04];
/// execute(Opcode::Delta, &payload, &mut output, Some(&reference));
/// ```
pub fn execute_delta(payload: &[u8], output_buffer: &mut [u8], reference_buffer: Option<&[u8]>) {
    let Some(ref_buf) = reference_buffer else {
        output_buffer.fill(0);
        return;
    };

    let max_buffer_len = output_buffer.len();
    let mut out_pos = 0;
    let mut patch_pos = 0;

    while patch_pos < payload.len() && out_pos < max_buffer_len {
        let instruction = payload[patch_pos];
        if instruction == 0x00 {
            break;
        }

        if instruction == 0x01 {
            if patch_pos + 4 >= payload.len() {
                break;
            }
            let ref_offset =
                u16::from_le_bytes([payload[patch_pos + 1], payload[patch_pos + 2]]) as usize;
            let copy_size =
                u16::from_le_bytes([payload[patch_pos + 3], payload[patch_pos + 4]]) as usize;

            let safe_size = core::cmp::min(copy_size, max_buffer_len - out_pos);
            let safe_ref_end = core::cmp::min(ref_offset + safe_size, ref_buf.len());
            let actual_size = safe_ref_end.saturating_sub(ref_offset);

            output_buffer[out_pos..out_pos + actual_size]
                .copy_from_slice(&ref_buf[ref_offset..safe_ref_end]);

            out_pos += actual_size;
            patch_pos += 5;
        } else if instruction == 0x02 {
            if patch_pos + 2 >= payload.len() {
                break;
            }
            let add_size =
                u16::from_le_bytes([payload[patch_pos + 1], payload[patch_pos + 2]]) as usize;
            let safe_size = core::cmp::min(add_size, max_buffer_len - out_pos);

            if patch_pos + 3 + safe_size > payload.len() {
                break;
            }

            output_buffer[out_pos..out_pos + safe_size]
                .copy_from_slice(&payload[patch_pos + 3..patch_pos + 3 + safe_size]);

            out_pos += safe_size;
            patch_pos += 3 + add_size;
        } else {
            break;
        }
    }
}

/// Executes the `Seed` opcode, which initializes the output buffer with a pseudo-random sequence based on the given seed and then applies patches from the payload.
/// # Arguments
/// * `param` - The seed value used to initialize the pseudo-random number generator.
/// * `payload` - The data containing the patch instructions.
/// * `output_buffer` - The buffer where the generated sequence and patches will be stored.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 16];
/// let payload = [0x01, 0x00, 0x00, 0x4B]; // 1 patch at offset 0 with value 'K'
/// execute(Opcode::Seed, 42, &payload, &mut output, None);
/// assert_eq!(output[0], b'K');
/// ```
pub fn execute_seed(param: u32, payload: &[u8], output_buffer: &mut [u8]) {
    let mut state = if param == 0 { 1 } else { param };

    for item in output_buffer.iter_mut() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *item = (state & 0xFF) as u8;
    }

    if payload.len() >= 2 {
        let num_patches = u16::from_le_bytes([payload[0], payload[1]]) as usize;
        let mut offset = 2;

        for _ in 0..num_patches {
            if offset + 3 > payload.len() {
                break;
            }
            let pos = u16::from_le_bytes([payload[offset], payload[offset + 1]]) as usize;
            if pos < output_buffer.len() {
                output_buffer[pos] = payload[offset + 2];
            }
            offset += 3;
        }
    }
}

/// Executes the `Dict` opcode, which reconstructs data by copying words from a reference dictionary based on indices provided in the payload.
///
/// # Arguments
/// * `param` - The size of each word in the dictionary.
/// * `payload` - The data containing the indices of the words to be copied from the dictionary.
/// * `output_buffer` - The buffer where the reconstructed data will be stored.
/// * `reference_buffer` - An optional buffer that serves as the reference dictionary for the operation.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 8];
/// let dict = b"AAAABBBB";
/// let payload = [1, 0]; // Indices of the words to copy from the dictionary
/// execute(Opcode::Dict, 4, &payload, &mut output, Some(dict));
/// assert_eq!(&output, b"BBBBAAAA");
/// ```
pub fn execute_dict(
    param: u32,
    payload: &[u8],
    output_buffer: &mut [u8],
    reference_buffer: Option<&[u8]>,
) {
    let dict_buf = match reference_buffer {
        Some(buf) if param > 0 && !buf.is_empty() => buf,
        _ => {
            output_buffer.fill(0);
            return;
        }
    };

    let word_size = param as usize;
    let mut out_pos = 0;
    let max_words = dict_buf.len() / word_size;
    let max_buffer_len = output_buffer.len();

    for &index_byte in payload {
        if out_pos >= max_buffer_len {
            break;
        }

        let index = index_byte as usize;
        let safe_size = core::cmp::min(word_size, max_buffer_len - out_pos);

        if index < max_words {
            let dict_offset = index * word_size;
            output_buffer[out_pos..out_pos + safe_size]
                .copy_from_slice(&dict_buf[dict_offset..dict_offset + safe_size]);
        } else {
            output_buffer[out_pos..out_pos + safe_size].fill(0);
        }
        out_pos += safe_size;
    }
}

/// Executes the `Xor` opcode, which performs a bitwise XOR operation on the payload with either a constant key derived from the `param` or with a reference buffer if provided.
///
/// The result is stored in the output buffer.
/// # Arguments
/// * `param` - A parameter that may be used to derive a constant key for the XOR operation.
/// * `payload` - The data to be `XORed`.
/// * `output_buffer` - The buffer where the result of the XOR operation will be stored.
/// * `reference_buffer` - An optional buffer that may be used for the XOR operation. If provided, the payload will be `XORed` with this buffer; otherwise, it will be `XORed` with a constant key derived from `param`.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 1];
/// let payload = [0b10101010];
/// execute(Opcode::Xor, 0b11111111, &payload, &mut output, None);
/// assert_eq!(output[0], 0b01010101);
/// ```
pub fn execute_xor(
    param: u32,
    payload: &[u8],
    output_buffer: &mut [u8],
    reference_buffer: Option<&[u8]>,
) {
    let len = core::cmp::min(payload.len(), output_buffer.len());

    if let Some(ref_buf) = reference_buffer {
        let safe_len = core::cmp::min(len, ref_buf.len());
        for i in 0..safe_len {
            output_buffer[i] = payload[i] ^ ref_buf[i];
        }
    } else {
        let key = (param & 0xFF) as u8;
        for i in 0..len {
            output_buffer[i] = payload[i] ^ key;
        }
    }
}
/// Executes the `Rs` opcode, which performs a right shift operation on each byte of the payload by a specified number of bits, and stores the result in the output buffer.
/// # Arguments
/// * `param` - A parameter that specifies the number of bits to shift (0-7).
/// * `payload` - The data to be shifted.
/// * `output_buffer` - The buffer where the result of the right shift operation will be stored.
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 1];
/// let payload = [0b10000000]; // 128
/// execute(Opcode::Rs, 1, &payload, &mut output, None);
/// assert_eq!(output[0], 0b01000000); // 64
/// ```
pub fn execute_rs(param: u32, payload: &[u8], output_buffer: &mut [u8]) {
    let len = core::cmp::min(payload.len(), output_buffer.len());
    let shift = (param & 0x07) as u8;

    if shift == 0 {
        output_buffer[..len].copy_from_slice(&payload[..len]);
    } else {
        for i in 0..len {
            output_buffer[i] = payload[i] >> shift;
        }
    }
}
/// The execute function applies the specified opcode to the output buffer, using the provided parameters and payload.
/// It also optionally uses a reference buffer for certain opcodes.
/// # Arguments
/// * `opcode` - The operation code to execute.
/// * `param` - A parameter that may be used by certain opcodes.
/// * `payload` - The data associated with the operation.
/// * `output_buffer` - The buffer where the result of the operation will be stored.
/// * `reference_buffer` - An optional buffer that may be used for certain operations, such as Delta
/// # Example
/// ```no_run
/// use kpack::{Opcode, execute};
/// let mut output = [0u8; 1024];
/// let payload = [1, 2, 3, 4];
/// execute(Opcode::Lit, 4, &payload, &mut output, None);
/// ```
pub fn execute(
    opcode: &Opcode,
    param: u32,
    payload: &[u8],
    output_buffer: &mut [u8],
    reference_buffer: Option<&[u8]>,
) {
    match opcode {
        Opcode::Lit => execute_lit(param, payload, output_buffer),
        Opcode::Delta => execute_delta(payload, output_buffer, reference_buffer),
        Opcode::Xor => execute_xor(param, payload, output_buffer, reference_buffer),
        Opcode::Rle => execute_rle(param, payload, output_buffer),
        Opcode::Seed => execute_seed(param, payload, output_buffer),
        Opcode::Dict => execute_dict(param, payload, output_buffer, reference_buffer),
        Opcode::Rs => execute_rs(param, payload, output_buffer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_eq;
    #[test]
    fn test_execute_lit() {
        // Préparation des données pures
        let mut payload = [0u8; 3576];
        payload[..5].copy_from_slice(b"Hello");
        let mut buffer = [0u8; 10];

        // Exécution déconnectée de tout Nœud
        execute(&Opcode::Lit, 5, &payload, &mut buffer, None);

        // Vérification
        assert_eq!(&buffer[..5], b"Hello");
    }

    #[test]
    fn test_execute_rle() {
        let mut payload = [0u8; 3576];
        payload[0] = b'X';
        let mut buffer = [0u8; 10];

        execute(&Opcode::Rle, 10, &payload, &mut buffer, None);

        assert_eq!(&buffer, b"XXXXXXXXXX");
    }

    #[test]
    fn test_execute_seed_and_patch() {
        let mut payload = [0u8; 3576];
        payload[0] = 0x01; // 1 patch
        payload[1] = 0x00;
        payload[2] = 0x00; // offset 0
        payload[3] = 0x00;
        payload[4] = 0x4B; // La lettre 'K'
        let mut buffer = [0u8; 16];

        execute(&Opcode::Seed, 42, &payload, &mut buffer, None);

        assert_eq!(buffer[0], b'K');
    }

    #[test]
    fn test_execute_delta() {
        let mut payload = [0u8; 3576];
        payload[0] = 0x01; // Instruction de copie depuis la référence
        payload[1] = 0x00; // offset = 0
        payload[2] = 0x00;
        payload[3] = 0x04; // taille = 4
        payload[4] = 0x00;
        payload[5] = 0x00; // Fin des instructions

        let parent = b"AmentysOSBest";
        let mut buffer = [0u8; 4];

        // On passe la référence au moteur
        execute(&Opcode::Delta, 0, &payload, &mut buffer, Some(parent));

        assert_eq!(&buffer, b"Amen");
    }

    #[test]
    fn test_execute_dict() {
        let mut payload = [0u8; 3576];
        payload[0] = 1; // On demande l'Index 1 (le 2ème mot)
        payload[1] = 0; // On demande l'Index 0 (le 1er mot)

        // Notre dictionnaire contient des mots de 4 octets
        let dict = b"AAAABBBB";
        let mut buffer = [0u8; 8];

        // param = 4 (taille du mot)
        execute(&Opcode::Dict, 4, &payload, &mut buffer, Some(dict));

        // Le résultat attendu est BBBB puis AAAA
        assert_eq!(&buffer, b"BBBBAAAA");
    }

    #[test]
    fn test_execute_xor() {
        let mut payload = [0u8; 3576];
        payload[0] = 0b10101010;
        let mut buffer = [0u8; 1];

        // Test sans référence : on XOR avec le param (0b11111111)
        execute(&Opcode::Xor, 0b11111111, &payload, &mut buffer, None);

        // Le XOR avec 1 inverse tous les bits
        assert_eq!(buffer[0], 0b01010101);
    }

    #[test]
    fn test_execute_rs() {
        let mut payload = [0u8; 3576];
        payload[0] = 0b10000000; // 128
        let mut buffer = [0u8; 1];

        // Décalage de 1 bit vers la droite (équivaut à une division par 2)
        execute(&Opcode::Rs, 1, &payload, &mut buffer, None);

        assert_eq!(buffer[0], 0b01000000); // 64
    }
    #[test]
    fn test_advanced_kpack_archive_reconstruction() {
        let mut archive_reconstruite = [0u8; 64];

        let payload_lit = b"AMENTYS-OS";
        // On écrit dans la tranche de 0 à 10
        execute(
            &Opcode::Lit,
            10,
            payload_lit,
            &mut archive_reconstruite[0..10],
            None,
        );

        let payload_rle = [0xAA]; // Le motif à répéter
        execute(
            &Opcode::Rle,
            20,
            &payload_rle,
            &mut archive_reconstruite[10..30],
            None,
        );

        let dictionnaire = b"COREDATABOOT";
        let payload_dict = [2, 0, 1];
        execute(
            &Opcode::Dict,
            4,
            &payload_dict,
            &mut archive_reconstruite[30..42],
            Some(dictionnaire),
        );

        let payload_xor = [0x11, 0x07, 0x01, 0x17, 0x10, 0x0B, 0x16, 0x1B];
        execute(
            &Opcode::Xor,
            0x42,
            &payload_xor,
            &mut archive_reconstruite[42..50],
            None,
        );

        let parent_delta = b"AmentysOSBase!";
        let payload_delta = [
            0x01, 0x00, 0x00, 0x09, 0x00, 0x02, 0x04, 0x00, b'C', b'o', b'r', b'e', 0x01, 0x0D,
            0x00, 0x01, 0x00, 0x00,
        ];
        execute(
            &Opcode::Delta,
            0,
            &payload_delta,
            &mut archive_reconstruite[50..64],
            Some(parent_delta),
        );

        assert_eq!(&archive_reconstruite[0..10], b"AMENTYS-OS", "Echec LIT");
        assert_eq!(&archive_reconstruite[10..30], &[0xAA; 20], "Echec RLE");
        assert_eq!(&archive_reconstruite[30..42], b"BOOTCOREDATA", "Echec DICT");
        assert_eq!(&archive_reconstruite[42..50], b"SECURITY", "Echec XOR");
        assert_eq!(
            &archive_reconstruite[50..64],
            b"AmentysOSCore!",
            "Echec DELTA"
        );
    }
}
