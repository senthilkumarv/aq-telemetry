// Mapper to convert domain models to Thrift types
use crate::domain::dashboard::Dashboard;
use crate::domain::telemetry::{ChartData, ChartKind, SeriesData, TileData};
use telemetry_thrift::{SDChart, SDOverlay, SDPage, SDPoint, SDSeries, SDTile};
use thrift::OrderedFloat;

pub fn dashboard_to_thrift(dashboard: Dashboard) -> SDPage {
    let tiles: Vec<SDTile> = dashboard
        .tiles
        .into_iter()
        .map(tile_to_thrift)
        .collect();

    let charts: Vec<SDChart> = dashboard
        .charts
        .into_iter()
        .map(chart_to_thrift)
        .collect();

    SDPage::new(
        Some(dashboard.title),
        Some(tiles),
        Some(charts),
        None::<Vec<SDOverlay>>,
    )
}

fn tile_to_thrift(tile: TileData) -> SDTile {
    SDTile::new(
        Some(tile.id),
        Some(tile.title),
        Some(tile.unit),
        Some(OrderedFloat::from(tile.value)),
        Some(tile.precision),
    )
}

fn chart_to_thrift(chart: ChartData) -> SDChart {
    let kind = match chart.kind {
        ChartKind::Line => Some(telemetry_thrift::ChartKind::LINE),
        ChartKind::MultiLine => Some(telemetry_thrift::ChartKind::MULTILINE),
    };

    let series: Vec<SDSeries> = chart
        .series
        .into_iter()
        .map(series_to_thrift)
        .collect();

    SDChart::new(
        Some(chart.id),
        Some(chart.title),
        chart.unit,
        kind,
        chart.y_min.map(OrderedFloat::from),
        chart.y_max.map(OrderedFloat::from),
        chart.fraction_digits,
        Some(series),
    )
}

fn series_to_thrift(series: SeriesData) -> SDSeries {
    let points: Vec<SDPoint> = series
        .points
        .into_iter()
        .map(|p| SDPoint::new(Some(p.time_ms), Some(OrderedFloat::from(p.value))))
        .collect();

    SDSeries::new(
        Some(series.id),
        Some(series.name),
        series.color,
        Some(points),
    )
}

