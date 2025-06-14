-- Create admin settings table
CREATE TABLE admin_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    setting_key VARCHAR(100) UNIQUE NOT NULL,
    setting_value JSONB NOT NULL,
    description TEXT,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default settings
INSERT INTO admin_settings (setting_key, setting_value, description) VALUES
('general', '{
    "siteName": "Ericsson Budhilaw",
    "siteDescription": "Senior Software Engineer specializing in consulting and freelancing",
    "maintenanceMode": false,
    "maintenanceMessage": "The site is currently under maintenance. Please check back later.",
    "photo_profile": null,
    "social_media_links": {
        "github": "https://github.com/budhilaw",
        "linkedin": "https://linkedin.com/in/budhilaw",
        "x": "https://x.com/ceritaeric",
        "facebook": "https://facebook.com/ceritaeric",
        "instagram": "https://instagram.com/ceritaeric",
        "email": "ericsson@budhilaw.com"
    },
    "files": {
        "resume_links": "https://drive.google.com/"
    }
}', 'General site configuration'),

('features', '{
    "commentsEnabled": true,
    "portfolioEnabled": true,
    "servicesEnabled": true,
    "blogEnabled": true,
    "contactFormEnabled": true,
    "searchEnabled": true
}', 'Feature toggle settings'),

('notifications', '{
    "emailNotifications": true,
    "newCommentNotifications": true,
    "newContactFormNotifications": true,
    "systemAlertNotifications": true
}', 'Notification preferences'),

('security', '{
    "requireStrongPasswords": true,
    "sessionTimeout": 60,
    "maxLoginAttempts": 5,
    "twoFactorEnabled": false,
    "ipWhitelist": []
}', 'Security configuration');

-- Indexes
CREATE INDEX idx_admin_settings_key ON admin_settings(setting_key);
CREATE INDEX idx_admin_settings_updated_at ON admin_settings(updated_at); 