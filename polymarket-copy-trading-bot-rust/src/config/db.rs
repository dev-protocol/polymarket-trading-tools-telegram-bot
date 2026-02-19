use anyhow::Result;
use mongodb::{Client, Database};
use colored::Colorize;

pub async fn connect_db(mongo_uri: &str) -> Result<Database> {
    let client = Client::with_uri_str(mongo_uri).await?;
    let db = client.database("polymarket_copytrading");
    println!("{} MongoDB connected", "✓".green());
    Ok(db)
}

pub async fn cleanup_database(
    db: &Database,
    proxy_wallet: &str,
    user_addresses: &[String],
) -> Result<()> {
    println!("{} Cleaning up database before startup...", "🧹".yellow());

    let proxy_wallet_lower = proxy_wallet.to_lowercase();
    
    // Clean up activities and positions for proxy wallet
    let activity_collection_name = format!("user_activities_{}", proxy_wallet_lower);
    let position_collection_name = format!("user_positions_{}", proxy_wallet_lower);
    
    let activity_collection = db.collection::<mongodb::bson::Document>(&activity_collection_name);
    let position_collection = db.collection::<mongodb::bson::Document>(&position_collection_name);
    
    let activity_result = activity_collection.delete_many(mongodb::bson::doc! {}, None).await?;
    let position_result = position_collection.delete_many(mongodb::bson::doc! {}, None).await?;
    
    let mut total_activities_deleted = activity_result.deleted_count;
    let mut total_positions_deleted = position_result.deleted_count;

    // Clean up for all tracked trader addresses
    for trader_address in user_addresses {
        let trader_address_lower = trader_address.to_lowercase();
        let trader_activity_collection_name = format!("user_activities_{}", trader_address_lower);
        let trader_position_collection_name = format!("user_positions_{}", trader_address_lower);
        
        let trader_activity_collection = db.collection::<mongodb::bson::Document>(&trader_activity_collection_name);
        let trader_position_collection = db.collection::<mongodb::bson::Document>(&trader_position_collection_name);
        
        let trader_activity_result = trader_activity_collection.delete_many(mongodb::bson::doc! {}, None).await?;
        let trader_position_result = trader_position_collection.delete_many(mongodb::bson::doc! {}, None).await?;
        
        total_activities_deleted += trader_activity_result.deleted_count;
        total_positions_deleted += trader_position_result.deleted_count;
    }

    println!(
        "{} Database cleanup completed: {} activities and {} positions removed",
        "✓".green(),
        total_activities_deleted,
        total_positions_deleted
    );

    Ok(())
}

#[allow(dead_code)] // May be used for future features
pub fn get_user_activity_collection(db: &Database, wallet_address: &str) -> mongodb::Collection<mongodb::bson::Document> {
    let collection_name = format!("user_activities_{}", wallet_address.to_lowercase());
    db.collection(&collection_name)
}

pub fn get_user_position_collection(db: &Database, wallet_address: &str) -> mongodb::Collection<mongodb::bson::Document> {
    let collection_name = format!("user_positions_{}", wallet_address.to_lowercase());
    db.collection(&collection_name)
}

