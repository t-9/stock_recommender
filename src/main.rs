use std::io::{self, BufRead, BufReader, Write};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use serde::{Deserialize};

#[tokio::main]
async fn main() {
    loop {
        print_menu();
        
        let mode: u32 = match scan_input().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        match mode {
            0 => break,
            3 => highest_rate_mode()
                .await,
            4 => score_mode()
                .await,
            5 => score_ranking_mode()
                .await,
            _ => (),
        }
    }
}

async fn highest_rate_mode() {
    print_input_ticker();
    let ticker = scan_input();
    print_highest_rate(ticker)
        .await
        .expect("Failed to get finance data");
}

async fn score_mode() {
    print_input_ticker();
    let ticker = scan_input();
    print_score(&ticker)
        .await
        .expect("Failed to get finance data");
}

async fn score_ranking_mode() {
    print_score_ranking()
        .await
        .expect("Failed to get finance data");
}

fn print_menu() {
    println!("");
    println!("Please Select Mode");
    println!("0: exit");
    println!("3: Highest Rate");
    println!("4: Score");
    println!("5: Score Ranking");
}

fn print_input_ticker() {
    println!("");
    println!("Please Input Ticker");
}

fn scan_input() -> String {
    print!("> ");
    io::stdout()
        .flush()
        .expect("Failed to flush");
    
    let mut val = String::new();
    io::stdin()
        .read_line(&mut val)
        .expect("Failed to read line");
    return val.trim().to_string();
}

#[derive(Debug)]
#[derive(Deserialize)]
struct ChartError {
    code: String,
    description: Option<String>,
}

impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl std::error::Error for ChartError {}

#[derive(Deserialize)]
struct ChartResultTime {
    timezone: String,
    start: u64,
    end: u64,
    gmtoffset: f64,
}

#[derive(Deserialize)]
struct ChartResultMetaCurrentTradingPeriod {
    pre: ChartResultTime,
    regular: ChartResultTime,
    post: ChartResultTime,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartResultMeta {
    currency: Option<String>,
    symbol: String,
    exchange_name: String,
    instrument_type: String,
    first_trade_date: Option<f64>,
    regular_market_time: u64,
    gmtoffset: f64,
    timezone: String,
    exchange_timezone_name: String,
    regular_market_price: f64,
    chart_previous_close: f64,
    price_hint: u64,
    current_trading_period: ChartResultMetaCurrentTradingPeriod,
    data_granularity: String,
    range: String,
    valid_ranges: Vec<String>,
}

#[derive(Deserialize)]
struct ChartResultIndicatorQuote {
    high: Vec<Option<f64>>,
    volume: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
    open: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct ChartResultIndicatorAdjclose {
    adjclose: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct ChartResultIndicators {
    quote: Vec<ChartResultIndicatorQuote>,
    adjclose: Vec<ChartResultIndicatorAdjclose>,
}

#[derive(Deserialize)]
struct ChartResult {
    meta: ChartResultMeta,
    timestamp: Vec<f64>,
    indicators: ChartResultIndicators,
}


#[derive(Deserialize)]
struct Chart {
    result: Option<Vec<ChartResult>>,
    error: Option<ChartError>,
}

#[derive(Deserialize)]
struct ChartResponse {
    chart: Chart,
}

async fn print_highest_rate(ticker: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("");
    let resp = reqwest::get(format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?range=10y&interval=1d&events=history", ticker))
        .await?
        .json::<ChartResponse>()
        .await?;
    
    match resp.chart.error {
        Some(error) => {
            println!("error: {}", error.code);
            return Ok(());
        },
        None => {}
    }

    let mut highest_counter = -1;
    let mut counter = -1;
    let mut highest_price = 0.;

    for price in resp.chart.result.unwrap()[0].indicators.adjclose[0].adjclose.iter() {
        match price {
            Some(price) => {
                if highest_price < *price {
                    println!("{:#?}", price);
                    highest_counter += 1;
                    highest_price = *price;
                }
                counter += 1;
            },
            None => {}
        }
    }
    let highest_rate = highest_counter as f64 / counter as f64;
    println!("{:#?}", highest_rate);
    Ok(())
}

fn rri(num_of_periods: f64, present_value: f64, future_value: f64) -> f64 {
    (future_value / present_value).powf(1. / num_of_periods) - 1.
}

async fn ticker_rri(ticker: &str, years: i16) -> Result<f64, Box<dyn std::error::Error>> {
    let resp_future = reqwest::get(format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}y&interval=1d&events=history", ticker, years));
    let resp = resp_future
        .await?
        .json::<ChartResponse>()
        .await?;
    
    if let Some(error) = resp.chart.error {
        return Err(Box::new(error));
    }

    let resp_reslut = resp.chart.result.unwrap();
    let mut pointer = 0;
    let mut previous_price = resp_reslut[0].indicators.adjclose[0].adjclose.first().unwrap().unwrap();
    while previous_price <= 0. {
        pointer += 1;
        previous_price = resp_reslut[0].indicators.adjclose[0].adjclose.get(pointer).unwrap().unwrap();
    }
    pointer = resp_reslut[0].indicators.adjclose[0].adjclose.len() - 1;
    let mut latest_price_option = resp_reslut[0].indicators.adjclose[0].adjclose.last();
    while latest_price_option.unwrap().is_none() {
        pointer -= 1;
        latest_price_option = resp_reslut[0].indicators.adjclose[0].adjclose.get(pointer);
    }
    let latest_price = latest_price_option.unwrap().unwrap();
    Ok(rri(years as f64, previous_price, latest_price))
}

async fn ticker_score(ticker: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let five_years_rri = ticker_rri(ticker, 5).await?;
    let ten_years_rri = ticker_rri(ticker, 10).await?;
    let quarter_century_rri = ticker_rri(ticker, 25).await?;
    Ok((five_years_rri + ten_years_rri + quarter_century_rri) / 3.)
}

async fn print_score(ticker: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("");
    let score = ticker_score(ticker).await;
    match score {
        Ok(score) => println!("{:#?}", score),
        Err(e) => println!("Error: {}", e),
    };
    Ok(())
}

async fn print_score_ranking() -> Result<(), Box<dyn std::error::Error>> {
    println!("");

    let mut tickers: Vec<String> = Vec::new();

    let f = File::open("ticker.txt").unwrap();
    let reader = BufReader::new(f);
    for line in reader.lines() {
        let line = line.unwrap();
        tickers.push(line.trim().to_string());
    }

    let mut scores = HashMap::new();
    let mut score_future_list = HashMap::new();
    for ticker in tickers.iter() {
        let score_future = ticker_score(ticker);
        score_future_list.insert(ticker, Box::pin(score_future));
    }

    let mut tmp_file = File::create("tmp.csv")?;
    for ticker in tickers.iter() {
        let score = score_future_list.get_mut(ticker).unwrap().await;
        match score {
            Ok(score) => {
                println!("{}: {:#?}", ticker, score);
                if let Err(e) = writeln!(tmp_file, "{},{:#?}", ticker, score) {
                    println!("{}", e)
                }
                scores.insert(ticker, score);
            },
            Err(e) => {
                println!("Error: {}", e);
                continue;
            },
        };
    }

    println!("");

    let mut sorted: Vec<_> = scores.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    let mut result_file = File::create("result.csv")?;
    let mut rank = 1;
    for (ticker, score) in sorted {
        println!("{},{},{:#?}", rank, ticker, score);
        if let Err(e) = writeln!(result_file, "{},{},{:#?}", rank, ticker, score) {
            println!("{}", e)
        }
        rank += 1;
    }

    Ok(())
}