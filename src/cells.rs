use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageKeySpec {
    pub offset: usize,
    pub len: usize,
}

/// Static analysis result from scanning a WASM bytecode file.
pub struct ManifestAnalysis {
    pub static_read_slots:  Vec<[u8; 32]>,
    pub static_write_slots: Vec<[u8; 32]>,
    pub has_storage_reads:  bool,
    pub has_storage_writes: bool,
    pub fully_resolved: bool,
}

pub struct CellAccount;

impl CellAccount {
    pub fn compute_manifest_hash(
        bytecode: &[u8],
        declared_reads: &[[u8; 32]],
        declared_writes: &[[u8; 32]],
        commutative_keys: &[[u8; 32]],
    ) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(bytecode);
        for slot in declared_reads {
            hasher.update(slot);
        }
        for slot in declared_writes {
            hasher.update(slot);
        }
        for slot in commutative_keys {
            hasher.update(slot);
        }
        *hasher.finalize().as_bytes()
    }

    pub fn analyze_bytecode(bytecode: &[u8]) -> Result<ManifestAnalysis, String> {
        use wasmparser::{Payload, Parser, Operator};
        use std::collections::HashMap;

        if bytecode.is_empty() {
            return Ok(ManifestAnalysis {
                static_read_slots:  vec![],
                static_write_slots: vec![],
                has_storage_reads:  false,
                has_storage_writes: false,
                fully_resolved: true,
            });
        }

        let mut import_map: HashMap<String, u32> = HashMap::new();
        let mut func_index = 0u32;
        let mut data_map: std::collections::BTreeMap<usize, Vec<u8>> = std::collections::BTreeMap::new();

        for payload in Parser::new(0).parse_all(bytecode) {
            match payload.map_err(|e| format!("WASM parse error: {}", e))? {
                Payload::ImportSection(imports) => {
                    for import in imports {
                        let import = import.map_err(|e| format!("Import error: {}", e))?;
                        if matches!(import.ty, wasmparser::TypeRef::Func(_)) {
                            import_map.insert(import.name.to_string(), func_index);
                            func_index += 1;
                        }
                    }
                }
                Payload::DataSection(data_section) => {
                    for segment in data_section {
                        let segment = segment.map_err(|e| format!("Data segment error: {}", e))?;
                        if let wasmparser::DataKind::Active { memory_index: 0, offset_expr } = segment.kind {
                            let mut reader = offset_expr.get_operators_reader();
                            if let Ok(Operator::I32Const { value }) = reader.read() {
                                let base = value as usize;
                                data_map.insert(base, segment.data.to_vec());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let read_slot = |ptr: usize| -> Option<[u8; 32]> {
            for (&base, bytes) in data_map.range(..=ptr).rev() {
                let rel = ptr.wrapping_sub(base);
                if rel + 32 <= bytes.len() {
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&bytes[rel..rel + 32]);
                    return Some(out);
                }
            }
            None
        };

        let storage_read_idx  = import_map.get("storage_read").copied();
        let storage_write_idx = import_map.get("storage_write").copied();

        let mut static_read_slots: Vec<[u8; 32]> = Vec::new();
        let mut static_write_slots: Vec<[u8; 32]> = Vec::new();
        let mut has_reads = false;
        let mut has_writes = false;
        let mut fully_resolved = true;

        for payload in Parser::new(0).parse_all(bytecode) {
            match payload.map_err(|e| format!("WASM parse error: {}", e))? {
                Payload::CodeSectionEntry(body) => {
                    let mut reader = body.get_operators_reader()
                        .map_err(|e| format!("Operator reader error: {}", e))?;
                    let mut stack: Vec<i32> = Vec::new();

                    while !reader.eof() {
                        let op = reader.read().map_err(|e| format!("Read error: {}", e))?;
                        match op {
                            Operator::I32Const { value } => stack.push(value),
                            Operator::Call { function_index } => {
                                if Some(function_index) == storage_read_idx {
                                    has_reads = true;
                                    if let Some(ptr) = stack.pop() {
                                        if let Some(slot) = read_slot(ptr as usize) {
                                            static_read_slots.push(slot);
                                        } else {
                                            fully_resolved = false;
                                        }
                                    } else {
                                        fully_resolved = false;
                                    }
                                } else if Some(function_index) == storage_write_idx {
                                    has_writes = true;
                                    if let Some(ptr) = stack.pop() {
                                        if let Some(slot) = read_slot(ptr as usize) {
                                            static_write_slots.push(slot);
                                        } else {
                                            fully_resolved = false;
                                        }
                                    } else {
                                        fully_resolved = false;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        static_read_slots.sort_unstable();
        static_read_slots.dedup();
        static_write_slots.sort_unstable();
        static_write_slots.dedup();

        Ok(ManifestAnalysis {
            static_read_slots,
            static_write_slots,
            has_storage_reads: has_reads,
            has_storage_writes: has_writes,
            fully_resolved,
        })
    }

    pub fn verify_manifest_against_bytecode(
        bytecode: &[u8],
        declared_reads: &[[u8; 32]],
        declared_writes: &[[u8; 32]],
        storage_key_specs: &[StorageKeySpec],
    ) -> Result<(), String> {
        if bytecode.is_empty() {
            return Ok(());
        }

        let analysis = Self::analyze_bytecode(bytecode)?;

        if analysis.has_storage_reads && declared_reads.is_empty() && storage_key_specs.is_empty() {
            return Err(
                "Bytecode calls storage_read but declared_reads is empty. Run `trth manifest-init`.".to_string(),
            );
        }
        if analysis.has_storage_writes && declared_writes.is_empty() && storage_key_specs.is_empty() {
            return Err(
                "Bytecode calls storage_write but declared_writes is empty. Run `trth manifest-init`.".to_string(),
            );
        }

        if analysis.fully_resolved && !analysis.static_read_slots.is_empty() {
            let declared_set: std::collections::HashSet<[u8; 32]> =
                declared_reads.iter().copied().collect();
            for slot in &analysis.static_read_slots {
                if !declared_set.contains(slot) {
                    return Err(format!(
                        "Bytecode reads slot {} not in declared_reads.",
                        hex::encode(slot)
                    ));
                }
            }
        }

        if analysis.fully_resolved && !analysis.static_write_slots.is_empty() {
            let declared_set: std::collections::HashSet<[u8; 32]> =
                declared_writes.iter().copied().collect();
            for slot in &analysis.static_write_slots {
                if !declared_set.contains(slot) {
                    return Err(format!(
                        "Bytecode writes slot {} not in declared_writes.",
                        hex::encode(slot)
                    ));
                }
            }
        }

        Ok(())
    }
}
