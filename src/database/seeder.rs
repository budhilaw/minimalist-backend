use anyhow::Result;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::utils::errors::AppError;

pub struct DatabaseSeeder {
    pool: PgPool,
}

impl DatabaseSeeder {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn seed_all(&self) -> Result<(), AppError> {
        tracing::info!("🌱 Starting database seeding...");

        // Clear existing data (in reverse order due to foreign keys)
        self.clear_data().await?;

        // Seed data in dependency order
        let user_ids = self.seed_users().await?;
        let post_ids = self.seed_posts(&user_ids).await?;
        let portfolio_ids = self.seed_portfolio_projects().await?;
        let service_ids = self.seed_services().await?;
        self.seed_comments(&post_ids, &user_ids).await?;
        self.seed_audit_logs(&user_ids).await?;

        tracing::info!("✅ Database seeding completed successfully!");
        tracing::info!(
            "📊 Seeded {} users, {} posts, {} portfolio projects, {} services",
            user_ids.len(),
            post_ids.len(),
            portfolio_ids.len(),
            service_ids.len()
        );

        Ok(())
    }

    async fn clear_data(&self) -> Result<(), AppError> {
        tracing::info!("🧹 Clearing existing data...");

        // Clear in reverse dependency order
        sqlx::query("DELETE FROM audit_logs")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM comments")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM services")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM portfolio_projects")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM posts").execute(&self.pool).await?;
        sqlx::query("DELETE FROM users").execute(&self.pool).await?;

        Ok(())
    }

    async fn seed_users(&self) -> Result<Vec<Uuid>, AppError> {
        tracing::info!("👥 Seeding users...");

        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(b"password123", &salt)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?
            .to_string();

        let mut user_ids = Vec::new();

        // Admin user
        let admin_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, full_name, phone, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(admin_id)
        .bind("admin")
        .bind("admin@portfolio.dev")
        .bind(&password_hash)
        .bind("Admin User")
        .bind("+1234567890")
        .bind("admin")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        user_ids.push(admin_id);

        // Regular users
        let john_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, full_name, phone, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(john_id)
        .bind("johndoe")
        .bind("john@example.com")
        .bind(&password_hash)
        .bind("John Doe")
        .bind("+1234567891")
        .bind("user")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        user_ids.push(john_id);

        let jane_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, full_name, phone, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(jane_id)
        .bind("janedoe")
        .bind("jane@example.com")
        .bind(&password_hash)
        .bind("Jane Doe")
        .bind("+1234567892")
        .bind("user")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        user_ids.push(jane_id);

        Ok(user_ids)
    }

    async fn seed_posts(&self, user_ids: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
        tracing::info!("📝 Seeding blog posts...");

        let mut post_ids = Vec::new();
        let author_id = user_ids[0]; // Admin user as author

        // Post 1: Getting Started with Rust
        let post1_id = Uuid::new_v4();
        let tags1 = vec![
            "rust".to_string(),
            "programming".to_string(),
            "tutorial".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO posts (id, title, slug, excerpt, content, category, tags, featured_image,
                             published, featured, author_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(post1_id)
        .bind("Getting Started with Rust")
        .bind("getting-started-with-rust")
        .bind("Learn the basics of Rust programming language")
        .bind("# Getting Started with Rust\n\nRust is a systems programming language...")
        .bind("Programming")
        .bind(&tags1)
        .bind(Some(
            "https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=800",
        ))
        .bind(true)
        .bind(true)
        .bind(author_id)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        post_ids.push(post1_id);

        // Post 2: Building a REST API with Axum
        let post2_id = Uuid::new_v4();
        let tags2 = vec![
            "rust".to_string(),
            "axum".to_string(),
            "api".to_string(),
            "web".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO posts (id, title, slug, excerpt, content, category, tags, featured_image,
                             published, featured, author_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(post2_id)
        .bind("Building a REST API with Axum")
        .bind("building-rest-api-axum")
        .bind("Complete guide to building REST APIs using the Axum web framework")
        .bind("# Building a REST API with Axum\n\nAxum is a modern, ergonomic web framework...")
        .bind("Web Development")
        .bind(&tags2)
        .bind(Some(
            "https://images.unsplash.com/photo-1516321318423-f06f85e504b3?w=800",
        ))
        .bind(true)
        .bind(false)
        .bind(author_id)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        post_ids.push(post2_id);

        Ok(post_ids)
    }

    async fn seed_portfolio_projects(&self) -> Result<Vec<Uuid>, AppError> {
        tracing::info!("💼 Seeding portfolio projects...");

        let mut project_ids = Vec::new();

        // Project 1: E-commerce Platform
        let project1_id = Uuid::new_v4();
        let tech1 = vec![
            "React".to_string(),
            "Node.js".to_string(),
            "PostgreSQL".to_string(),
            "Stripe".to_string(),
            "AWS".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO portfolio_projects (id, title, slug, description, long_description, category, technologies, 
                                          live_url, github_url, image_url, featured, active, status, start_date, end_date, client, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#
        )
        .bind(project1_id)
        .bind("E-commerce Platform")
        .bind("ecommerce-platform")
        .bind("A full-stack e-commerce solution built with modern technologies")
        .bind(Some(r#"# E-commerce Platform

A comprehensive e-commerce solution built with modern technologies, featuring a complete shopping experience from browsing to checkout.

## Key Features

- **User Authentication**: Secure login/registration with JWT tokens
- **Product Catalog**: Dynamic product listings with search and filtering
- **Shopping Cart**: Real-time cart management with persistent storage
- **Payment Integration**: Stripe payment processing with webhooks
- **Admin Dashboard**: Complete order and inventory management

## Technical Implementation

### Frontend
- Built with **React 18** and **TypeScript**
- State management using **Redux Toolkit**
- Responsive design with **Tailwind CSS**
- Real-time updates via **WebSocket** connections

### Backend
- **Node.js** with **Express** framework
- **PostgreSQL** database with **Prisma** ORM
- **Redis** for session management and caching
- **AWS S3** for image storage

## Screenshots

![Homepage](https://images.unsplash.com/photo-1556742049-0cfed4f6a45d?w=800)

## Challenges & Solutions

One of the main challenges was implementing real-time inventory updates across multiple user sessions. We solved this by:

1. Using WebSocket connections for live updates
2. Implementing optimistic UI updates
3. Adding conflict resolution for concurrent purchases

## Results

- **50% increase** in conversion rate
- **30% reduction** in cart abandonment
- **99.9% uptime** since launch

> "This platform transformed our online presence and significantly boosted our sales." - Client Testimonial"#))
        .bind("Web Application")
        .bind(&tech1)
        .bind(Some("https://ecommerce-demo.example.com"))
        .bind(Some("https://github.com/user/ecommerce-platform"))
        .bind(Some("https://images.unsplash.com/photo-1556742049-0cfed4f6a45d?w=800"))
        .bind(true)
        .bind(true) // active
        .bind("completed")
        .bind(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap())
        .bind(Some(NaiveDate::from_ymd_opt(2023, 6, 30).unwrap()))
        .bind(Some("Acme Corp"))
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        project_ids.push(project1_id);

        // Project 2: Task Management API
        let project2_id = Uuid::new_v4();
        let tech2 = vec![
            "Rust".to_string(),
            "Axum".to_string(),
            "PostgreSQL".to_string(),
            "Redis".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO portfolio_projects (id, title, slug, description, long_description, category, technologies, 
                                          live_url, github_url, image_url, featured, active, status, start_date, end_date, client, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#
        )
        .bind(project2_id)
        .bind("Task Management API")
        .bind("task-management-api")
        .bind("RESTful API for team task management and collaboration")
        .bind(Some(r#"# Task Management API

A high-performance REST API built with Rust and Axum, designed for team collaboration and task management with enterprise-grade features.

## Architecture Overview

This API follows a clean architecture pattern with clear separation of concerns:

```
┌─────────────────┐
│   Handlers      │ ← HTTP Layer
├─────────────────┤
│   Services      │ ← Business Logic
├─────────────────┤
│  Repositories   │ ← Data Access
├─────────────────┤
│   Database      │ ← PostgreSQL
└─────────────────┘
```

## Key Features

### Authentication & Authorization
- **JWT-based authentication** with refresh tokens
- **Role-based access control** (Admin, Manager, User)
- **API key authentication** for service-to-service communication

### Task Management
- Create, update, delete, and assign tasks
- **Priority levels** and **due date tracking**
- **File attachments** with cloud storage integration
- **Task dependencies** and **subtask support**

### Real-time Features
- **WebSocket notifications** for task updates
- **Live collaboration** on task comments
- **Real-time dashboard** updates

### Advanced Filtering & Search
- **Full-text search** with PostgreSQL
- **Complex filtering** by multiple criteria
- **Sorting and pagination** for large datasets

## Technical Stack

- **Language**: Rust 🦀
- **Framework**: Axum (async web framework)
- **Database**: PostgreSQL with SQLx
- **Caching**: Redis for session management
- **Authentication**: JWT with RS256 signing
- **Documentation**: OpenAPI/Swagger integration

## Performance Metrics

- **Response time**: < 50ms average
- **Throughput**: 10,000+ requests/second
- **Memory usage**: < 50MB under load
- **CPU efficiency**: 99% utilization

## API Endpoints

### Tasks
```http
GET    /api/v1/tasks              # List tasks with filtering
POST   /api/v1/tasks              # Create new task
GET    /api/v1/tasks/{id}         # Get task details
PUT    /api/v1/tasks/{id}         # Update task
DELETE /api/v1/tasks/{id}         # Delete task
```

### Teams
```http
GET    /api/v1/teams              # List teams
POST   /api/v1/teams              # Create team
GET    /api/v1/teams/{id}/members # Get team members
```

## Code Example

```rust
#[derive(Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub assignee_id: Option<Uuid>,
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, AppError> {
    let task = state.task_service
        .create_task(payload)
        .await?;
    
    Ok(Json(task.into()))
}
```

## Testing

- **Unit tests**: 95% code coverage
- **Integration tests**: All endpoints tested
- **Load testing**: Handles 10k concurrent users
- **Security testing**: OWASP compliance

## Deployment

Deployed using Docker containers with:
- **Kubernetes** orchestration
- **Horizontal pod autoscaling**
- **Health checks** and **monitoring**
- **CI/CD pipeline** with GitHub Actions"#))
        .bind("Backend API")
        .bind(&tech2)
        .bind(Option::<String>::None)
        .bind(Some("https://github.com/user/task-api"))
        .bind(Some("https://images.unsplash.com/photo-1611224923853-80b023f02d71?w=800"))
        .bind(false)
        .bind(false) // active - this one will be inactive
        .bind("completed")
        .bind(NaiveDate::from_ymd_opt(2023, 7, 1).unwrap())
        .bind(Some(NaiveDate::from_ymd_opt(2023, 12, 15).unwrap()))
        .bind(Some("Tech Startup"))
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        project_ids.push(project2_id);

        Ok(project_ids)
    }

    async fn seed_services(&self) -> Result<Vec<Uuid>, AppError> {
        tracing::info!("🛠️ Seeding services...");

        let mut service_ids = Vec::new();

        // Service 1: Full-Stack Web Development
        let service1_id = Uuid::new_v4();
        let features1 = vec![
            "Custom web applications".to_string(),
            "Responsive design".to_string(),
            "Database design".to_string(),
            "API development".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO services (id, title, description, features, category, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(service1_id)
        .bind("Full-Stack Web Development")
        .bind("Complete web application development from frontend to backend")
        .bind(&features1)
        .bind("Web Development")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        service_ids.push(service1_id);

        // Service 2: API Development
        let service2_id = Uuid::new_v4();
        let features2 = vec![
            "REST API design".to_string(),
            "Database optimization".to_string(),
            "Authentication".to_string(),
        ];
        sqlx::query(
            r#"
            INSERT INTO services (id, title, description, features, category, active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(service2_id)
        .bind("API Development & Integration")
        .bind("RESTful API development and third-party service integration")
        .bind(&features2)
        .bind("Backend Development")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        service_ids.push(service2_id);

        Ok(service_ids)
    }

    async fn seed_comments(&self, post_ids: &[Uuid], _user_ids: &[Uuid]) -> Result<(), AppError> {
        tracing::info!("💬 Seeding comments...");

        // Comment 1
        let comment1_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO comments (id, post_id, parent_id, author_name, author_email, 
                                content, status, ip_address, user_agent, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10, $11)
            "#,
        )
        .bind(comment1_id)
        .bind(post_ids[0])
        .bind(Option::<Uuid>::None)
        .bind("John Doe")
        .bind("john@example.com")
        .bind("Great introduction to Rust! I've been meaning to learn it for a while.")
        .bind("approved")
        .bind("192.168.1.1")
        .bind(Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"))
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        // Comment 2 (reply to comment 1)
        let comment2_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO comments (id, post_id, parent_id, author_name, author_email, 
                                content, status, ip_address, user_agent, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10, $11)
            "#,
        )
        .bind(comment2_id)
        .bind(post_ids[0])
        .bind(Some(comment1_id))
        .bind("Admin")
        .bind("admin@portfolio.dev")
        .bind("Thanks for the feedback! I'm glad you found it helpful.")
        .bind("approved")
        .bind("192.168.1.100")
        .bind(Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"))
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn seed_audit_logs(&self, user_ids: &[Uuid]) -> Result<(), AppError> {
        tracing::info!("📋 Seeding audit logs...");

        let audit_logs = vec![
            (user_ids[0], "login", "users"),
            (user_ids[0], "create", "posts"),
            (user_ids[1], "register", "users"),
            (user_ids[0], "create", "portfolio_projects"),
        ];

        for (user_id, action, resource_type) in audit_logs {
            sqlx::query(
                r#"
                INSERT INTO audit_logs (id, user_id, action, resource_type, ip_address, user_agent, created_at)
                VALUES ($1, $2, $3, $4, $5::inet, $6, $7)
                "#
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(action)
            .bind(resource_type)
            .bind("192.168.1.100")
            .bind("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}
