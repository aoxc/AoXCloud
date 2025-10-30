use sqlx::postgres::PgPoolOptions;
use std::env;
use std::path::Path;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar logging
    tracing_subscriber::fmt::init();

    // Cargar variables de entorno (primero .env.local, luego .env)
    if let Ok(path) = env::var("DOTENV_PATH") {
        dotenv::from_path(Path::new(&path)).ok();
    } else {
        dotenv::from_filename(".env.local").ok();
        dotenv::dotenv().ok();
    }

    // Obtener DATABASE_URL desde variables de entorno
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL debe estar configurada");

    println!("Conectando a la base de datos...");

    // Crear pool de conexiones
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;

    // Ejecutar migraciones
    println!("Ejecutando migraciones...");

    // Obtenemos el directorio desde una variable de entorno o usamos un valor por defecto
    let migrations_dir = env::var("MIGRATIONS_DIR").unwrap_or_else(|_| "./migrations".to_string());
    println!("Directorio de migraciones: {}", migrations_dir);

    // Crear un migrator
    let migrator = sqlx::migrate::Migrator::new(Path::new(&migrations_dir))
        .await
        .expect("No se pudo crear el migrator");

    // Ejecutar todas las migraciones pendientes
    migrator.run(&pool).await?;

    println!("Migraciones aplicadas correctamente");

    Ok(())
}
