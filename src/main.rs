use bedrockrs::level::level::level::{BedrockLevel, ChunkSelectionFilter, GetBlockConfig, LevelConfiguration, LevelDbError};
use bedrockrs::core::world::dimension::Dimension;
use bedrockrs::level::level::sub_chunk::{SubChunk, SubChunkSerDe, SerDeStore, SubChunkTraitExtended, SubChunkTrait};
use std::path::Path;

// file io
use std::fs::File;
use std::fs;
use serde_json;
use std::io::{self, Write, BufWriter, BufReader, BufRead};
use bedrockrs::level::level::db_interface::bedrock_key::ChunkKey;
use bedrockrs::level::level::db_interface::rusty::DBError;
use bedrockrs::level::level::file_interface::RawWorldTrait;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    scan_limit_x: i32,
    scan_limit_z: i32,
}


// loads the world and returns a BedrockLevel
fn load_world() -> Result<BedrockLevel, LevelDbError> {
    BedrockLevel::open(
        Box::from(Path::new("./db")),
        LevelConfiguration::default(),
        &mut (),
    )
}

// fetches
fn fetch_subchunk(x: i32, y: i32, z: i32, bedrock_level: &mut BedrockLevel) -> SubChunk {
    println!("subchunk coordinates {},{},{}", x, y, z);
    let sub_chunk = bedrock_level
        .get_sub_chunk::<SubChunk, SubChunkSerDe>(
            (x,y,z).into(),
            Dimension::Overworld,
            SerDeStore::default(),
        )
        .expect("Read failed")
        .unwrap_or_else(|| SubChunk::empty((x, y, z).into(), Dimension::Overworld).to_init());
    return sub_chunk;
}

fn construct_vector_data(subchunk_data:Vec<SubChunk>){
    let file_path: String = format!("./csvs/{}_{}.csv", subchunk_data[0].position().x, subchunk_data[0].position().z);
    let file = File::create(&file_path).unwrap();
    let mut writer = BufWriter::new(file);
    let mut chunk_scan: Vec<(i32, i32, i32, String)> = Vec::new();

    for subchunk in subchunk_data{
        let chunk_position = subchunk.position();
        for y in 0u8..16{
            for x in 0u8..16{
                for z in 0u8..16{
                    let block_data = subchunk.get_block((x,y,z).into());
                    if block_data.unwrap().name != "minecraft:air"{
                        chunk_scan.push(((x as i32)+(16* chunk_position.x), (y as i32)+(16* chunk_position.y), (z as i32)+(16* chunk_position.z), block_data.unwrap().name.clone()));
                    }
                }
            }
        }
    }
    for block in chunk_scan {
        writer.write_all((format!("{:?},{:?},{:?},{:?}\n", block.0, block.1, block.2, block.3)).as_bytes()).unwrap();
        writer.flush().unwrap();
    }
    let metadata = fs::metadata(&file_path).unwrap();
    if metadata.len() == 0 {
        fs::remove_file(&file_path).unwrap();
    }
}

fn main() {
    let mut bedrock_level = load_world().unwrap();

    let dir_path = Path::new("./csvs");
    fs::create_dir_all(dir_path).unwrap();

    let config_str = fs::read_to_string("./chunker.json").expect("Failed to load config!");
    let config:Config = serde_json::from_str(&config_str).expect("Failed to parse config!");
    println!("
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⡀⠀⠀⠀⠀
⠀⠀⠀⠀⢀⡴⣆⠀⠀⠀⠀⠀⣠⡀⠀⠀⠀⠀⠀⠀⣼⣿⡗⠀⠀⠀⠀
⠀⠀⠀⣠⠟⠀⠘⠷⠶⠶⠶⠾⠉⢳⡄⠀⠀⠀⠀⠀⣧⣿⠀⠀⠀⠀⠀
⠀⠀⣰⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⣤⣤⣤⣤⣤⣿⢿⣄⠀⠀⠀⠀
⠀⠀⡇⠀⢀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣧⠀⠀⠀⠀⠀⠀⠙⣷⡴⠶⣦
⠀⠀⢱⡀⠀⠉⠉⠀⠀⠀⠀⠛⠃⠀⢠⡟⠂⠀⠀⢀⣀⣠⣤⠿⠞⠛⠋
⣠⠾⠋⠙⣶⣤⣤⣤⣤⣤⣀⣠⣤⣾⣿⠴⠶⠚⠋⠉⠁⠀⠀⠀⠀⠀⠀
⠛⠒⠛⠉⠉⠀⠀⠀⣴⠟⣣⡴⠛⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠛⠛⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
    ");
    println!("[MurkMapChunker] Loaded config, scanning range: {},{}", config.scan_limit_x, config.scan_limit_z);

    let mut progress:f32 = 0.0;
    for limit_x in -config.scan_limit_x..config.scan_limit_x{
        for limit_z in -config.scan_limit_z..config.scan_limit_z{
            let mut subchunk_data:Vec<SubChunk> = Vec::new();
            let mut subchunk_identifier:Vec<String> = Vec::new();
            for x in limit_x..limit_x+1 {
                for z in limit_z..limit_z+1 {
                    for y in (-4..20).rev() {
                        let chunk_key = ChunkKey::new_sub_chunk((x, y, z).into(), Dimension::Overworld);
                        {
                            let level_db_raw = bedrock_level.underlying_world_interface();
                            let chunk_raw = level_db_raw.get_sub_chunk_raw(chunk_key);
                            println!("{:?}", chunk_raw);
                            match chunk_raw {
                                Ok(None) => {
                                }
                                Ok(Some(chunk)) => {
                                    subchunk_data.push(fetch_subchunk(x, y, z, &mut bedrock_level));
                                    subchunk_identifier.push(format!("{}_{}", x, z));
                                }
                                Err(e) => {}
                            }
                        }
                    }
                }
            }
            if !subchunk_data.is_empty() {
                construct_vector_data(subchunk_data);
            }
        }
    }
}