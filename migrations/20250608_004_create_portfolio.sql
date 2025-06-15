CREATE TABLE portfolio_projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT NOT NULL,
    long_description TEXT,
    category VARCHAR(50) NOT NULL,
    technologies TEXT[] NOT NULL DEFAULT '{}',
    live_url VARCHAR(500),
    github_url VARCHAR(500),
    image_url VARCHAR(500),
    featured BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT true,
    status VARCHAR(20) NOT NULL DEFAULT 'completed',
    start_date DATE NOT NULL,
    end_date DATE,
    client VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_portfolio_slug ON portfolio_projects(slug);
CREATE INDEX idx_portfolio_category ON portfolio_projects(category);
CREATE INDEX idx_portfolio_status ON portfolio_projects(status);
CREATE INDEX idx_portfolio_featured ON portfolio_projects(featured);
CREATE INDEX idx_portfolio_active ON portfolio_projects(active);
CREATE INDEX idx_portfolio_technologies ON portfolio_projects USING GIN(technologies);

-- Trigger
CREATE TRIGGER update_portfolio_updated_at BEFORE UPDATE ON portfolio_projects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column(); 