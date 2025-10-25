use std::fs::File;
use std::io::Read;

use deadpool_postgres::{Manager, Pool, Runtime};
use tokio_postgres::{config::SslMode, Config, NoTls};

use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;

pub async fn pg_pool() -> Result<Pool, String> {
    let pg_conn = std::env::var("POSTGRES_CONN")
        .map_err(|_| format!("Postgres connection string missing"))?;

    let pool = create_pool(&pg_conn).await;
    _ = pool
        .get()
        .await
        .map_err(|e| format!("Db connection verification failed: {e}"))?;

    Ok(pool)
}

async fn create_pool(config: &str) -> Pool {
    let pg_config = config.parse::<Config>().expect("Bad pg config string");

    let manager = match pg_config.get_ssl_mode() {
        SslMode::Require => {
            let mut cert_file =
                File::open("/opt/ca-certificate.crt").expect("Failed to open certificate file");
            let mut cert_data = Vec::new();
            cert_file
                .read_to_end(&mut cert_data)
                .expect("Failed to read certificate file");

            let certificate =
                Certificate::from_pem(&cert_data).expect("Failed to parse certificate");

            let tls_connector = TlsConnector::builder()
                .add_root_certificate(certificate)
                .build()
                .expect("Failed to create TLS connector");
            let tls = MakeTlsConnector::new(tls_connector);
            Manager::new(pg_config, tls)
        }
        _ => Manager::new(pg_config, NoTls),
    };

    let pool = Pool::builder(manager)
        .runtime(Runtime::Tokio1)
        .build()
        .expect("Runtime was not set?");

    pool
}
