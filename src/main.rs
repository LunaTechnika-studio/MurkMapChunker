use bedrockrs::level::level::level::{BedrockLevel, LevelConfiguration, LevelDbError};
use bedrockrs::core::world::dimension::Dimension;
use bedrockrs::level::level::sub_chunk::{SubChunk, SubChunkSerDe, SerDeStore, SubChunkTraitExtended, SubChunkTrait};
use std::path::Path;

// file io
use std::fs::File;
use std::fs;
use serde_json;
use std::io::{Write};
use bedrockrs::level::level::db_interface::bedrock_key::ChunkKey;
use bedrockrs::level::level::file_interface::RawWorldTrait;
use serde::{Serialize, Deserialize};
use bincode;
use flate2::Compression;
use flate2::write::GzEncoder;

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
    match bedrock_level.get_sub_chunk::<SubChunk, SubChunkSerDe>(
        (x, y, z).into(),
        Dimension::Overworld,
        SerDeStore::default(),
    ) {
        Ok(Some(sub_chunk)) => sub_chunk, // Successfully fetched the sub-chunk
        Ok(None) => {
            // Handle the case where the sub-chunk is missing (but no error occurred)
            println!("Sub-chunk is missing, returning empty sub-chunk.");
            SubChunk::empty((x, y, z).into(), Dimension::Overworld).to_init()
        }
        Err(e) => {
            // Handle the case where fetching failed
            println!("Error fetching sub-chunk: {:?}", e);
            SubChunk::empty((x, y, z).into(), Dimension::Overworld).to_init()
        }
    }
}

fn construct_vector_data(subchunk_data:Vec<SubChunk>){
    let file_path: String = format!("./chunk_binary_data/{}_{}.mmbf", subchunk_data[0].position().x, subchunk_data[0].position().z);
    let mut file = File::create(&file_path).unwrap();
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
    let serelialized_chunk = bincode::serialize(&chunk_scan).unwrap();
    let mut encoder = GzEncoder::new(file, Compression::fast());
    encoder.write_all(&serelialized_chunk).unwrap();


    let metadata = fs::metadata(&file_path).unwrap();
    if metadata.len() == 0 {
        fs::remove_file(&file_path).unwrap();
    }
}

fn main() {
    let mut bedrock_level = load_world().unwrap();

    let dir_path = Path::new("./chunk_binary_data");
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