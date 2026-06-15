#[cfg(test)]
mod teste {
    use crate::codegen::reg_alloc::{LinearScan, LiveInterval};

    #[test]
    fn test_happy_path_no_spill() {
        let mut allocator = LinearScan::new();

        let intervals = vec![
            LiveInterval {
                temp: 0,
                start: 1,
                end: 4,
            },
            LiveInterval {
                temp: 1,
                start: 2,
                end: 3,
            },
        ];

        allocator.allocate_register(intervals);

        //ambos devem coseguir registradores físicos sem estourar
        assert_eq!(allocator.reg_map.len(), 2); // qtd de reg
        assert_eq!(allocator.stack_map.len(), 0); // qtd no stack
    }

    #[test]
    fn test_register_recycling() {
        let mut allocator = LinearScan::new();

        let intervals = vec![
            LiveInterval {
                temp: 0,
                start: 1,
                end: 2,
            },
            LiveInterval {
                temp: 1,
                start: 3,
                end: 5,
            },
        ];

        allocator.allocate_register(intervals);

        let reg_t0 = allocator.reg_map.get(&0).unwrap();
        let reg_t1 = allocator.reg_map.get(&1).unwrap();
        assert_eq!(reg_t0, reg_t1);
    }

    #[test]
    fn test_linear_scan_spill() {
        let mut allocator = LinearScan::new();

        let intervals = vec![
            LiveInterval {
                temp: 0,
                start: 1,
                end: 10,
            },
            LiveInterval {
                temp: 1,
                start: 2,
                end: 5,
            },
            LiveInterval {
                temp: 2,
                start: 3,
                end: 6,
            },
            LiveInterval {
                temp: 3,
                start: 4,
                end: 7,
            },
            LiveInterval {
                temp: 4,
                start: 5,
                end: 6,
            },
        ];

        allocator.allocate_register(intervals);

        assert_eq!(allocator.reg_map.len(), 4);
        assert_eq!(allocator.stack_map.len(), 1);

        assert!(allocator.stack_map.contains_key(&0));
    }
}
