use nexus_sdk::{
    compile::CompileOpts,
    nova::seq::{Generate, Nova, PP},
    Local, Prover, Verifiable,
};
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

type Input = alloc::string::String;
type Output = bool;

const PACKAGE: &str = "guest";

use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Cell {
    Empty,
    Z,
    K,
}

struct Board {
    cells: [Cell; 9],
}

struct Player {
    symbol: Cell,
}

struct SimpleRNG {
    state: u64,
}

struct GameRound {
    seed: u64,
    player_moves: Vec<usize>,
}

impl SimpleRNG {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        SimpleRNG { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn rand_range(&mut self, min: usize, max: usize) -> usize {
        (self.next() % (max - min + 1) as u64) as usize + min
    }
}

impl Board {
    fn new() -> Board {
        Board {
            cells: [Cell::Empty; 9],
        }
    }

    fn make_move(&mut self, position: usize, player: Cell) -> bool {
        if position < 9 && self.cells[position] == Cell::Empty {
            self.cells[position] = player;
            true
        } else {
            false
        }
    }

    fn is_full(&self) -> bool {
        self.cells.iter().all(|&cell| cell != Cell::Empty)
    }

    fn check_winner(&self) -> Option<Cell> {
        const WINNING_COMBINATIONS: [[usize; 3]; 8] = [
            [0, 1, 2], [3, 4, 5], [6, 7, 8], // Rows
            [0, 3, 6], [1, 4, 7], [2, 5, 8], // Columns
            [0, 4, 8], [2, 4, 6],            // Diagonals
        ];

        for combo in WINNING_COMBINATIONS.iter() {
            if self.cells[combo[0]] != Cell::Empty
                && self.cells[combo[0]] == self.cells[combo[1]]
                && self.cells[combo[1]] == self.cells[combo[2]]
            {
                return Some(self.cells[combo[0]]);
            }
        }
        None
    }

    fn get_empty_cells(&self) -> Vec<usize> {
        self.cells.iter().enumerate()
            .filter(|(_, &cell)| cell == Cell::Empty)
            .map(|(index, _)| index)
            .collect()
    }
}

fn play_game(human: &Player, computer: &Player, rng: &mut SimpleRNG)  -> GameRound {
    let mut board = Board::new();
    let mut current_player = &human.symbol;
    let seed = rng.state;
    let mut player_moves = Vec::new();

        loop {
        display_board(&board);

        let position = if *current_player == human.symbol {
            let move_position = get_human_move(&board);
            player_moves.push(move_position);
            move_position
        } else {
            get_computer_move(&board, rng)
        };

        if board.make_move(position, *current_player) {
            if let Some(winner) = board.check_winner() {
                display_board(&board);
                if winner == human.symbol {
                    println!("You win!");
                } else {
                    println!("Computer wins!");
                }
                break;
            }

            if board.is_full() {
                display_board(&board);
                println!("It's a draw!");
                break;
            }

            current_player = if *current_player == human.symbol { &computer.symbol } else { &human.symbol };
        } else {
            println!("Invalid move. Try again.");
        }
    }
    
    GameRound {
        seed,
        player_moves,
    }
}

fn display_board(board: &Board) {
    for i in 0..3 {
        for j in 0..3 {
            let cell = match board.cells[i * 3 + j] {
                Cell::Empty => (i * 3 + j).to_string(),
                Cell::Z => "Z".to_string(),
                Cell::K => "K".to_string(),
            };
            print!("{}", cell);
            if j < 2 {
                print!("|");
            }
        }
        println!();
        if i < 2 {
            println!("-+-+-");
        }
    }
    println!();
}

fn get_human_move(board: &Board) -> usize {
    loop {
        println!("Enter your move (0-8):");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        match input.trim().parse() {
            Ok(num) if num < 9 && board.cells[num] == Cell::Empty => return num,
            _ => println!("Invalid move. Please enter a number between 0 and 8 for an empty cell."),
        }
    }
}

fn get_computer_move(board: &Board, rng: &mut SimpleRNG) -> usize {
    let empty_cells = board.get_empty_cells();
    let random_index = rng.rand_range(0, empty_cells.len() - 1);
    empty_cells[random_index]
}

fn format_seed_and_moves(seed: u64, moves: &[usize]) -> String {
    let mut result = seed.to_string();
    for &m in moves {
        result.push(',');
        result.push_str(&m.to_string());
    }
    result
}

fn main() {
    println!("Setting up Nova public parameters...");
    let pp: PP = PP::generate().expect("failed to generate parameters");

    let mut opts = CompileOpts::new(PACKAGE);
    opts.set_memlimit(8); // use an 8mb memory

    println!("Compiling guest program...");
    let prover: Nova<Local> = Nova::compile(&opts).expect("failed to compile guest program");

    println!("Welcome to ZiK-ZaK-Zoo!");
    let human = Player { symbol: Cell::Z };
    let computer = Player { symbol: Cell::K };
    let mut rng = SimpleRNG::new();

    let game_round = play_game(&human, &computer, &mut rng);

    println!("\nGame Round Data:");
    println!("Seed used: {}", game_round.seed);
    println!("Player moves: {:?}", game_round.player_moves);

    let input = format_seed_and_moves(game_round.seed, &game_round.player_moves);

    println!("Proving execution of vm...");
    let proof = prover
        .prove_with_input::<Input>(&pp, &input)
        .expect("failed to prove program");
    println!("Try and get output...");
    let output = proof.output::<Output>().expect("failed to deserialize output");
    
    println!(
        " output is {}!",
        output
    );
 
    println!(">>>>> Logging\n{}<<<<<", proof.logs().join("\n"));
 
    print!("Verifying execution...");
    proof.verify(&pp).expect("failed to verify proof");
 
    println!("  Succeeded!");
    // Print, notice, after committing to a journal, the private input became public
    println!("Wow it's {} that you won at ZiK-ZaK-ZoO!", output);    
}