-- Players Table
CREATE TABLE players (
    player_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_name VARCHAR(255) UNIQUE,
    player_email VARCHAR(255) UNIQUE,
    login_version INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Daily Puzzle Results Table
CREATE TABLE daily_puzzle_results (
    result_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID REFERENCES players(player_id),
    daily_puzzle INT NOT NULL,
    human_player INT NOT NULL,
    success BOOLEAN NOT NULL DEFAULT false,
    UNIQUE(player_id, daily_puzzle)
);

-- Daily Puzzle Attempts Table
CREATE TABLE daily_puzzle_attempts (
    attempt_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    result_id UUID REFERENCES daily_puzzle_results(result_id),
    sequence_of_moves TEXT NOT NULL DEFAULT '',
    move_count INT NOT NULL DEFAULT 0,
    won BOOLEAN NOT NULL DEFAULT false,
    attempt_number INT NOT NULL,
    attempt_started TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
