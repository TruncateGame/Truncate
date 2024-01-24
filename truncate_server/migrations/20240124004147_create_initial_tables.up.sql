-- Players Table
CREATE TABLE players (
    player_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_name VARCHAR(255) UNIQUE,
    login_version INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP WITH TIME ZONE
);

-- Daily Puzzle Results Table
CREATE TABLE daily_puzzle_results (
    result_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID REFERENCES players(player_id),
    daily_puzzle INT NOT NULL,
    success BOOLEAN NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    total_moves INT NOT NULL DEFAULT 0,
    UNIQUE(player_id, daily_puzzle)
);

-- Daily Puzzle Attempts Table
CREATE TABLE daily_puzzle_attempts (
    attempt_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    result_id UUID REFERENCES daily_puzzle_results(result_id),
    sequence_of_moves JSON NOT NULL,
    won BOOLEAN NOT NULL
);
