-- Maps Pera-to-Pera network (Section 7 & 16)
CREATE TABLE IF NOT EXISTS internal_graph_edges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    sender_user_id UUID NOT NULL,
    receiver_user_id UUID NOT NULL,
    
    -- Total value/volume of transfers between them
    total_tx_count INT NOT NULL DEFAULT 1,
    total_value_minor BIGINT NOT NULL DEFAULT 0,
    
    -- Calculated score (Section 3)
    relationship_score REAL NOT NULL DEFAULT 0.0,
    
    -- 'clean', 'risky', 'mule_activity'
    status VARCHAR(50) NOT NULL DEFAULT 'clean',
    
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_sender
        FOREIGN KEY(sender_user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_receiver
        FOREIGN KEY(receiver_user_id) 
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT unique_edge
        UNIQUE(sender_user_id, receiver_user_id)
);