use reqwest;
use serde::{Deserialize, de::DeserializeOwned};

pub struct GrocyCredentials {
    pub api_key: String,
    pub url: String,
}

struct Endpoints;
impl Endpoints {
    const SHOPPING_LIST_ITEMS: &'static str = "api/objects/shopping_list";
    const PRODUCTS: &'static str = "api/objects/products";
    const QUANTITY_UNITS: &'static str = "api/objects/quantity_units";
}


#[derive(Debug, Deserialize)]
pub struct ShoppingListItem {
    pub id: usize,
    pub product_id: usize,
    pub shopping_list_id: usize,
    pub note: String,
    pub amount: f64,
    pub done: usize,
    #[serde(rename = "qu_id")]
    pub quantity_unit_id: usize,
    row_created_timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct QuantityUnit {
    pub id: usize,
    pub name: String,
    pub description: String,
    pub name_plural: String,
    pub plural_forms: Option<usize>,

    #[serde(skip_deserializing)]
    pub active: usize,

    #[serde(skip_deserializing)]
    row_created_timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct Product {
    pub id: usize,
    pub name: String,

    #[serde(skip_deserializing)]
    pub description: String,

    #[serde(skip_deserializing)]
    pub product_group_id: usize,

    #[serde(skip_deserializing)]
    pub active: usize,

    #[serde(skip_deserializing)]
    pub location_id: usize,

    #[serde(skip_deserializing)]
    pub shopping_location_id: usize,
    
    #[serde(skip_deserializing)]
    pub qu_id_purchase: usize,

    #[serde(skip_deserializing)]
    pub qu_id_stock: usize,

    #[serde(skip_deserializing)]
    pub min_stock_amount: f64,

    #[serde(skip_deserializing)]
    pub default_best_before_days: usize,

    #[serde(skip_deserializing)]
    pub default_best_before_days_after_open: usize,

    #[serde(skip_deserializing)]
    pub default_best_before_days_after_freezing: usize,

    #[serde(skip_deserializing)]
    pub default_best_before_days_after_thawing: usize,
    
    #[serde(skip_deserializing)]
    pub picture_file_name: Option<String>,

    #[serde(skip_deserializing)]
    pub enable_tare_weight_handling: usize,

    #[serde(skip_deserializing)]
    pub tare_weight: usize,

    #[serde(skip_deserializing)]
    pub not_check_stock_fulfillment_for_recipes: usize,

    #[serde(skip_deserializing)]
    pub parent_product_id: usize,

    #[serde(skip_deserializing)]
    pub calories: usize,

    #[serde(skip_deserializing)]
    pub cumulate_min_stock_amount_of_sub_products: usize,

    #[serde(skip_deserializing)]
    pub due_type: usize,

    #[serde(skip_deserializing)]
    pub quick_consume_amount: f64,

    #[serde(skip_deserializing)]
    pub hide_on_stock_overview: usize,

    #[serde(skip_deserializing)]
    pub default_stock_label_type: usize,

    #[serde(skip_deserializing)]
    pub should_not_be_frozen: usize,

    #[serde(skip_deserializing)]
    pub treat_opened_as_out_of_stock: usize,

    #[serde(skip_deserializing)]
    pub no_own_stock: usize,

    #[serde(skip_deserializing)]
    pub default_consume_location_id: usize,

    #[serde(skip_deserializing)]
    pub move_on_open: usize,

    #[serde(skip_deserializing)]
    pub row_created_timestamp: String,
    
    #[serde(skip_deserializing)]
    pub qu_id_consume: usize,

    #[serde(skip_deserializing)]
    pub auto_reprint_stock_label: usize,

    #[serde(skip_deserializing)]
    pub quick_open_amount: f64,

    #[serde(skip_deserializing)]
    pub qu_id_price: usize,
}

pub async fn connect(api_key: String) -> Result<reqwest::Client, reqwest::Error> {
    let builder = reqwest::ClientBuilder::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let mut auth_value = reqwest::header::HeaderValue::from_str(&api_key.as_str())
        .expect("Failed to insert 'GROCY-API-KEY' header");
    auth_value.set_sensitive(true);
    headers.insert("GROCY-API-KEY", auth_value);

    builder.default_headers(headers).build()
}

pub async fn get_shopping_list_items(
    cred: &GrocyCredentials,
) -> Result<Vec<ShoppingListItem>, reqwest::Error> {
    let res: Vec<ShoppingListItem> = fetch_all(cred, Endpoints::SHOPPING_LIST_ITEMS).await?;
    Ok(res)
}

pub async fn get_product_name (
    id: usize, 
    cred: &GrocyCredentials,
) -> Result<String, reqwest::Error> {
    let res : Product = fetch_single(cred, Endpoints::PRODUCTS, id).await?; 
    Ok(res.name)
}

pub async fn delete_shopping_list_item (
    id: usize, 
    cred: &GrocyCredentials,
) -> Result<(), reqwest::Error> {
    println!("Deleting item {} from Grocy", id);
    delete_single(cred, Endpoints::SHOPPING_LIST_ITEMS, id).await?; 
    Ok(())
}

pub async fn get_quantity_unit (
    id: usize, 
    cred: &GrocyCredentials,
) -> Result<String, reqwest::Error> {
    let res : Product = fetch_single(cred, Endpoints::QUANTITY_UNITS, id).await?; 
    Ok(res.name)
}

async fn fetch_single<T>(cred: &GrocyCredentials, endpoint: &str, id :usize) -> Result<T, reqwest::Error> 
where T: DeserializeOwned
{
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&cred.url)
        .expect("Failed  to parse URL")
        .join(endpoint)
        .expect("Failed to join endpoint");
    println!("Fetch {:?}", url);
    let url = format!("{}/{}/{}", &cred.url, endpoint, id);

    println!("Endpoint: {}", endpoint);
    //let url = reqwest::Url::parse(&cred.url)
    //    .expect("Failed  to parse URL")
    //    .join(endpoint)
    //    .expect("Failed to join endpoint")
    //    .join(&id.to_string())
    //    .expect("Failed to join product id");
    
    println!("Fetch {:?}", url);
    let res: T = client
        .get(reqwest::Url::parse(&url).expect(""))
        .header("GROCY-API-KEY", cred.api_key.clone())
        .send()
        .await?
        .json()
        .await?;
    Ok(res)
}

async fn delete_single(cred: &GrocyCredentials, endpoint: &str, id :usize) -> Result<(), reqwest::Error> 
{
    let client = reqwest::Client::new();
    let url = format!("{}/{}/{}", &cred.url, endpoint, id);
    println!("DELETE {:?}", url);
    let res = client
        .delete(reqwest::Url::parse(&url).expect(""))
        .header("GROCY-API-KEY", cred.api_key.clone())
        .send()
        .await?;
    Ok(())
}

pub async fn fetch_all<T>(cred: &GrocyCredentials, endpoint: &str) -> Result<Vec<T>, reqwest::Error> 
where T: DeserializeOwned
{
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&cred.url)
        .expect("Failed  to parse URL")
        .join(endpoint)
        .expect("Failed to join endpoint");
    println!("Fetch {:?}", url);
    let res: Vec<T> = client
        .get(url)
        .header("GROCY-API-KEY", cred.api_key.clone())
        .send()
        .await?
        .json()
        .await?;
    Ok(res)
}
