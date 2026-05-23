#[test]
fn test_nmap_unsafe_mapping(){
    use std::fs::File;
    use std::io::Write;
    use crate::common::input::source::{SourceData, SourceFile};

    let path = std::env::temp_dir().join("test_nmap_crusty.c");
    {
        let mut file = File::create(&path).expect("Num deu pra criar o arquivo!");
        writeln!(file, "int x = 42;").expect("Num deu pra escrever no arquivo!");
    }

    let sf = SourceFile::from_path(path.clone()).expect("Falha ao mapear o arquivo pelo Mmap");

    match sf.source{
        SourceData::Mapped(_) => println!("Deu bão: O arquivo está mapeado em memória virtual!"),
        SourceData::Memory(_) => panic!("Deu ruim: O sistema usou String em vez do Mmap!"),
    }

    assert_eq!(sf.source.as_str().trim(), "int x = 42;");

    let _ = std::fs::remove_file(path);

}