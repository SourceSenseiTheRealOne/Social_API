CREATE TABLE likes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    content_type VARCHAR(50) NOT NULL,
    content_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_user_content UNIQUE (user_id, content_type, content_id)
);

CREATE INDEX idx_likes_content ON likes (content_type, content_id);
CREATE INDEX idx_likes_user_timeline ON likes (user_id, created_at DESC, id DESC);
CREATE INDEX idx_likes_created_at ON likes (created_at);
CREATE INDEX idx_likes_content_created ON likes (content_type, content_id, created_at);

CREATE TABLE like_counts (
    content_type VARCHAR(50) NOT NULL,
    content_id UUID NOT NULL,
    count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (content_type, content_id)
);

CREATE INDEX idx_like_counts_leaderboard ON like_counts (content_type, count DESC);
