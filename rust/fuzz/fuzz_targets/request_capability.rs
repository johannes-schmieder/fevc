#![no_main]

use std::mem::size_of;
use std::ptr;

use libfuzzer_sys::fuzz_target;
use vckss_plugin::ffi_engine::{
    vckss_rust_backend_request_capability_v1, vckss_rust_backend_request_capability_v2,
    vckss_rust_backend_request_capability_v3, VckssBackendRequestCapabilityReceiptV1,
    VckssBackendRequestCapabilityReceiptV2, VckssBackendRequestCapabilityReceiptV3,
    VckssBackendRequestCapabilityRequestV1, VckssBackendRequestCapabilityRequestV2,
    VckssBackendRequestCapabilityRequestV3,
};

struct Input<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Input<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.bytes.is_empty() {
            return 0;
        }
        let value = self.bytes[self.cursor % self.bytes.len()];
        self.cursor = self.cursor.wrapping_add(1);
        value
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes([self.byte(), self.byte(), self.byte(), self.byte()])
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes([
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
            self.byte(),
        ])
    }

    fn capacity<T>(&mut self) -> u32 {
        let size = u32::try_from(size_of::<T>()).expect("ABI structure size fits u32");
        match self.byte() % 5 {
            0 => 0,
            1 => size.saturating_sub(1),
            2 => size,
            3 => size.saturating_add(1),
            _ => self.u32(),
        }
    }
}

fn mutate_v1(input: &mut Input<'_>, request: &mut VckssBackendRequestCapabilityRequestV1) {
    request.algorithm = input.u32();
    request.deletion_mode = input.u32();
    request.nuisance_mode = input.u32();
    request.solver_route = input.u32();
    request.rng_contract = input.u32();
    request.controls_count = input.u32();
    request.frequency_use = input.u32();
    if input.byte() & 1 != 0 {
        request.abi_version = input.u32();
        request.struct_size = input.u32();
        request.request_schema = input.u32();
        request.reserved = input.u64();
    }
}

fn mutate_v2(input: &mut Input<'_>, request: &mut VckssBackendRequestCapabilityRequestV2) {
    mutate_v1(input, &mut request.v1);
    request.engine = input.u32();
    request.batch_mode = input.u32();
    request.stayers_mode = input.u32();
    request.target_weight_mode = input.u32();
    request.deletion_unit_source = input.u32();
    request.probeorder_supplied = input.u32();
    request.wallseconds_supplied = input.u32();
    request.physical_limit = input.u64();
    if input.byte() & 1 != 0 {
        request.reserved_2 = input.u32();
    }
}

fn mutate_v3(input: &mut Input<'_>, request: &mut VckssBackendRequestCapabilityRequestV3) {
    mutate_v2(input, &mut request.v2);
    request.leverage_batch_mode = input.u32();
    request.target_batch_mode = input.u32();
    request.allow_automatic_cmg_setup_fallback = input.u32();
    request.wallseconds = f64::from_bits(input.u64());
    if input.byte() & 1 != 0 {
        request.reserved_3 = input.u32();
        request.reserved_4 = input.u64();
    }
}

fuzz_target!(|data: &[u8]| {
    let mut input = Input::new(data);

    let mut request_v1 = VckssBackendRequestCapabilityRequestV1::default();
    mutate_v1(&mut input, &mut request_v1);
    let mut output_v1 = VckssBackendRequestCapabilityReceiptV1::default();
    let request_v1_ptr = if input.byte() & 1 == 0 {
        &request_v1
    } else {
        ptr::null()
    };
    let output_v1_ptr = if input.byte() & 1 == 0 {
        &mut output_v1
    } else {
        ptr::null_mut()
    };
    let capacity_v1 = input.capacity::<VckssBackendRequestCapabilityReceiptV1>();
    let _ = vckss_rust_backend_request_capability_v1(request_v1_ptr, output_v1_ptr, capacity_v1);
    let _ = vckss_rust_backend_request_capability_v1(
        &request_v1,
        &mut output_v1,
        u32::try_from(size_of::<VckssBackendRequestCapabilityReceiptV1>())
            .expect("V1 receipt size"),
    );

    let mut request_v2 = VckssBackendRequestCapabilityRequestV2::default();
    mutate_v2(&mut input, &mut request_v2);
    let mut output_v2 = VckssBackendRequestCapabilityReceiptV2::default();
    let request_v2_ptr = if input.byte() & 1 == 0 {
        &request_v2
    } else {
        ptr::null()
    };
    let output_v2_ptr = if input.byte() & 1 == 0 {
        &mut output_v2
    } else {
        ptr::null_mut()
    };
    let capacity_v2 = input.capacity::<VckssBackendRequestCapabilityReceiptV2>();
    let _ = vckss_rust_backend_request_capability_v2(request_v2_ptr, output_v2_ptr, capacity_v2);
    let _ = vckss_rust_backend_request_capability_v2(
        &request_v2,
        &mut output_v2,
        u32::try_from(size_of::<VckssBackendRequestCapabilityReceiptV2>())
            .expect("V2 receipt size"),
    );

    let mut request_v3 = VckssBackendRequestCapabilityRequestV3::default();
    mutate_v3(&mut input, &mut request_v3);
    let mut output_v3 = VckssBackendRequestCapabilityReceiptV3::default();
    let request_v3_ptr = if input.byte() & 1 == 0 {
        &request_v3
    } else {
        ptr::null()
    };
    let output_v3_ptr = if input.byte() & 1 == 0 {
        &mut output_v3
    } else {
        ptr::null_mut()
    };
    let capacity_v3 = input.capacity::<VckssBackendRequestCapabilityReceiptV3>();
    let _ = vckss_rust_backend_request_capability_v3(request_v3_ptr, output_v3_ptr, capacity_v3);
    let _ = vckss_rust_backend_request_capability_v3(
        &request_v3,
        &mut output_v3,
        u32::try_from(size_of::<VckssBackendRequestCapabilityReceiptV3>())
            .expect("V3 receipt size"),
    );
});
