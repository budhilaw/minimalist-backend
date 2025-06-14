-- Add user notification preferences and read status tracking
-- Up Migration

-- Table to store which notifications each user has read
CREATE TABLE user_notification_reads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    audit_log_id UUID NOT NULL REFERENCES audit_logs(id) ON DELETE CASCADE,
    read_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, audit_log_id)
);

-- Index for faster lookups
CREATE INDEX idx_user_notification_reads_user_id ON user_notification_reads(user_id);
CREATE INDEX idx_user_notification_reads_audit_log_id ON user_notification_reads(audit_log_id);
CREATE INDEX idx_user_notification_reads_read_at ON user_notification_reads(read_at);

-- Table to store user notification preferences
CREATE TABLE user_notification_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notification_type VARCHAR(100) NOT NULL, -- 'login', 'post_created', 'error', etc.
    enabled BOOLEAN NOT NULL DEFAULT true,
    delivery_method VARCHAR(50) NOT NULL DEFAULT 'in_app', -- 'in_app', 'email', 'both'
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, notification_type)
);

-- Index for faster lookups
CREATE INDEX idx_user_notification_preferences_user_id ON user_notification_preferences(user_id);
CREATE INDEX idx_user_notification_preferences_type ON user_notification_preferences(notification_type);

-- Insert default notification preferences for existing users
INSERT INTO user_notification_preferences (user_id, notification_type, enabled, delivery_method)
SELECT 
    u.id,
    notification_type,
    true,
    'in_app'
FROM users u
CROSS JOIN (
    VALUES 
    ('login'),
    ('logout'),
    ('post_created'),
    ('post_updated'),
    ('post_published'),
    ('portfolio_created'),
    ('portfolio_updated'),
    ('service_created'),
    ('service_updated'),
    ('comment_approved'),
    ('comment_rejected'),
    ('settings_updated'),
    ('profile_updated'),
    ('error'),
    ('warning'),
    ('system_alert')
) AS default_types(notification_type); 