mod common;

use std::{process::{Command, Stdio, Child}, thread, io::{BufReader, BufWriter, BufRead, Write, stdout}, collections::HashMap, sync::mpsc::{Receiver, Sender, channel}};

use cadabra::{Position};
use common::{test_positions::TEST_POSITIONS, load_config};

const RUN_REDUCED: bool = false; 

fn debug_perft(pos: &Position, depth: u8) -> HashMap<String, u64> {
    assert!(depth > 0);

    let moves = pos.generate_moves();

    let mut result: HashMap<String, u64> = HashMap::new();

    for m in moves {
        let mut copy = *pos;
        copy.make_move(m);
        let sub_nodes = if depth > 1 {
            copy.perft::<false>(depth - 1)
        } else {
            1
        };
        result.insert(m.to_uci_string(), sub_nodes);
    }

    result
}

#[test]
#[ignore]
fn run_perft_tests() {
    let config = load_config();
    let ref_engine_path = config.get("reference_engine_path").expect("please provide 'reference_engine_path' in cfg");
    
    let (mut send_task, mut recv_result, handle) = {
        let (send_task, recv_task) = channel();
        let (send_result, recv_result) = channel();

        let path = ref_engine_path.clone();
        let handle = thread::spawn(move || {
            let ref_engine = Command::new(path)
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn()
                                    .expect("Could not launch reference engine. Check path.");

            ref_engine_loop(ref_engine, (send_result, recv_task))
        });

        (send_task, recv_result, handle)
    };

    let positions = TEST_POSITIONS.iter().take(if RUN_REDUCED {24} else {TEST_POSITIONS.len()});

    for (name, fen, mut depth) in positions {
        if RUN_REDUCED {
            depth -= 1;
        }

        print!(" {name} at depth {depth} ... ");
        stdout().flush().unwrap();

        for depth in 1..=depth {
            if let Err((err, pos)) = validate_position(fen.to_string(), name, depth, false, (&mut send_task, &mut recv_result)) {
                println!("Error at {name}:\n{err}\n");
                pos.pretty_print();
                
                assert!(false)
            }
        }

        println!("\tok")
    }

    send_task.send(("close".to_string(), 0)).unwrap();

    handle.join().unwrap();

    println!("Validated all test positions")
}

fn ref_engine_loop(mut ref_engine: Child, (send_result, recv_task): (Sender<HashMap<String, u64>>, Receiver<(String, u8)>)) {
    let ref_in = ref_engine.stdin.take().unwrap();
    let ref_out = ref_engine.stdout.take().unwrap();

    let mut reader = BufReader::new(ref_out);
    let mut writer = BufWriter::new(ref_in);

    loop {
        let buffer = reader.fill_buf().unwrap();
        let length = buffer.len();
        reader.consume(length);
        
        let (fen, depth) = match recv_task.recv() {
            Ok((s, _)) if s == "close" => break,
            Ok(rec) => rec,
            Err(_) => todo!(),
        };

        writeln!(writer, "{}", format!("position fen {}", fen)).unwrap();
        writeln!(writer, "{}", format!("go perft {}", depth)).unwrap();
        writer.flush().unwrap();

        let mut results = HashMap::new();
        loop {
            let mut buf = String::new();

            reader.read_line(&mut buf).unwrap();

            let split: Vec<&str> = buf.split(':').collect();
            if split.len() != 2 {
                break;
            }
            results.insert(split[0].trim().to_string(), split[1].trim().parse::<u64>().unwrap());
        }
        send_result.send(results).unwrap();
    }
}

fn validate_position(fen: String, name: &str, depth: u8, tracing: bool, (send_task, recv_result): (&mut Sender<(String, u8)>, &mut Receiver<HashMap<String, u64>>)) -> Result<(), (String, Position)> {
    assert!(depth >= 1);
    // Reference engine io

    send_task.send((fen.clone(), depth)).unwrap();

    let mut pos = Position::from_fen(fen.as_str()).unwrap();
    let own_res = debug_perft(&pos, depth);

    let ref_res = recv_result.recv().unwrap();

    if depth == 1 {
        let missed_moves = ref_res.iter().filter(|m| !own_res.contains_key(m.0)).map(|m| m.0).collect::<Vec<&String>>();
        if missed_moves.len() > 0 {
            return Err((format!("Missed {} legal: {missed_moves:?}", missed_moves.len()), pos))
        }

        let extra_moves = own_res.iter().filter(|m| !ref_res.contains_key(m.0)).map(|m| m.0).collect::<Vec<&String>>();
        if extra_moves.len() > 0 {
            return Err((format!("Found {} too many: {extra_moves:?}", extra_moves.len()), pos))
        }

        if tracing {
            return Err((format!("This is weird, Probably an error in the move generator"), pos));
        }
    } else {
        for (key, nodes) in ref_res {
            if nodes != *own_res.get(&key).unwrap() {
                pos.make_uci_move(&key).unwrap();
                println!("Wrong move count on {name} at depth {depth}! Tracing with {key}");
                return validate_position(pos.get_fen_string(), name, depth - 1, true, (send_task, recv_result));
            };
        }
    }
    Ok(())
}