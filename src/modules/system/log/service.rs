//! 操作日志服务

use crate::entity;
use crate::error::AppResult;
use crate::modules::system::log::dto::*;
use crate::pagination::PageData;
use sea_orm::*;

/// 操作日志服务
pub struct LogService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> LogService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 操作日志分页列表
    pub async fn get_page(&self, q: &LogQuery) -> AppResult<PageData<LogVO>> {
        let mut s = entity::sys_log::Entity::find();
        if let Some(m) = q.module {
            s = s.filter(entity::sys_log::Column::Module.eq(m));
        }
        if let Some(a) = q.action_type {
            s = s.filter(entity::sys_log::Column::ActionType.eq(a));
        }
        if let Some(st) = q.status {
            s = s.filter(entity::sys_log::Column::Status.eq(st));
        }
        let total = s.clone().count(self.db).await?;
        let rows = s
            .order_by_desc(entity::sys_log::Column::CreateTime)
            .paginate(self.db, q.page_size as u64)
            .fetch_page((q.page_num - 1) as u64)
            .await?;
        Ok(PageData::new(
            rows.iter()
                .map(|r| LogVO {
                    id: r.id,
                    module: r.module,
                    action_type: r.action_type,
                    title: r.title.clone(),
                    content: r.content.clone(),
                    operator_id: r.operator_id,
                    operator_name: r.operator_name.clone(),
                    request_uri: r.request_uri.clone(),
                    request_method: r.request_method.clone(),
                    ip: r.ip.clone(),
                    province: r.province.clone(),
                    city: r.city.clone(),
                    device: r.device.clone(),
                    os: r.os.clone(),
                    browser: r.browser.clone(),
                    status: r.status,
                    error_msg: r.error_msg.clone(),
                    execution_time: r.execution_time,
                    create_time: r
                        .create_time
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                })
                .collect(),
            total,
        ))
    }

    /// 访问汇总
    pub async fn get_visit_summary(&self) -> AppResult<serde_json::Value> {
        use chrono::{Datelike, Local};

        let now = Local::now().naive_local().date();
        let today = now.and_hms_opt(0, 0, 0).unwrap();
        let tomorrow = today + chrono::Duration::days(1);
        let yesterday = today - chrono::Duration::days(1);

        let column = entity::sys_log::Column::CreateTime;
        let total_pv = entity::sys_log::Entity::find().count(self.db).await?;
        let today_pv = entity::sys_log::Entity::find()
            .filter(column.gte(today))
            .filter(column.lt(tomorrow))
            .count(self.db)
            .await?;
        let yesterday_pv = entity::sys_log::Entity::find()
            .filter(column.gte(yesterday))
            .filter(column.lt(today))
            .count(self.db)
            .await?;

        let ip = entity::sys_log::Column::Ip;
        let total_uv = entity::sys_log::Entity::find()
            .group_by(ip.clone())
            .count(self.db)
            .await?;
        let today_uv = entity::sys_log::Entity::find()
            .filter(column.gte(today))
            .filter(column.lt(tomorrow))
            .group_by(ip.clone())
            .count(self.db)
            .await?;
        let yesterday_uv = entity::sys_log::Entity::find()
            .filter(column.gte(yesterday))
            .filter(column.lt(today))
            .group_by(ip)
            .count(self.db)
            .await?;

        fn rate(today: u64, yesterday: u64) -> Option<f64> {
            if yesterday == 0 {
                None
            } else {
                Some(((today as f64 - yesterday as f64) / yesterday as f64 * 10000.0).round() / 10000.0)
            }
        }

        let _ = now.num_days_from_ce();
        Ok(serde_json::json!({
            "todayUv": today_uv,
            "yesterdayUv": yesterday_uv,
            "totalUv": total_uv,
            "todayPv": today_pv,
            "yesterdayPv": yesterday_pv,
            "totalPv": total_pv,
            "uvGrowthRate": rate(today_uv, yesterday_uv),
            "pvGrowthRate": rate(today_pv, yesterday_pv),
        }))
    }

    /// 访问趋势
    pub async fn get_visit_trend(&self, start: &str, end: &str) -> AppResult<serde_json::Value> {
        use chrono::NaiveDate;

        let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::Validation("日期格式不正确".into()))?;
        let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::Validation("日期格式不正确".into()))?;
        if start_date > end_date {
            return Err(crate::error::AppError::Validation("开始日期不能晚于结束日期".into()));
        }
        if (end_date - start_date).num_days() >= 366 {
            return Err(crate::error::AppError::Validation("查询跨度不能超过366天".into()));
        }

        let start_dt = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_dt = (end_date + chrono::Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();
        let rows = entity::sys_log::Entity::find()
            .filter(entity::sys_log::Column::CreateTime.gte(start_dt))
            .filter(entity::sys_log::Column::CreateTime.lt(end_dt))
            .all(self.db)
            .await?;

        let mut pv_map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut uv_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for log in &rows {
            if let Some(ct) = log.create_time {
                let day = ct.date().to_string();
                *pv_map.entry(day.clone()).or_insert(0) += 1;
                if let Some(ip) = &log.ip {
                    uv_map.entry(day).or_default().insert(ip.clone());
                }
            }
        }

        let mut points = Vec::new();
        let mut cursor = start_date;
        while cursor <= end_date {
            let day = cursor.to_string();
            points.push(serde_json::json!({
                "date": day,
                "pv": pv_map.get(&day).copied().unwrap_or(0),
                "uv": uv_map.get(&day).map(|s| s.len() as u64).unwrap_or(0),
            }));
            cursor += chrono::Duration::days(1);
        }

        Ok(serde_json::json!({
            "startDate": start,
            "endDate": end,
            "points": points,
        }))
    }

    /// 用户汇总
    pub async fn get_user_summary(&self) -> AppResult<serde_json::Value> {
        let total = entity::sys_user::Entity::find()
            .filter(entity::sys_user::Column::IsDeleted.eq(false))
            .count(self.db)
            .await?;
        Ok(serde_json::json!({ "total": total }))
    }
}
