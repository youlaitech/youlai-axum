//! 操作日志服务

use sea_orm::*;
use crate::common::pagination::PageData;
use crate::entity;
use crate::framework::web::error::AppResult;
use crate::modules::system::log::dto::*;

/// 操作日志服务
pub struct LogService<'a> { db: &'a DatabaseConnection }

impl<'a> LogService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    /// 操作日志分页列表
    pub async fn get_page(&self, q: &LogQuery) -> AppResult<PageData<LogVO>> {
        let mut s = entity::sys_log::Entity::find();
        if let Some(m) = q.module { s = s.filter(entity::sys_log::Column::Module.eq(m)); }
        if let Some(a) = q.action_type { s = s.filter(entity::sys_log::Column::ActionType.eq(a)); }
        if let Some(st) = q.status { s = s.filter(entity::sys_log::Column::Status.eq(st)); }
        let total = s.clone().count(self.db).await?;
        let rows = s.order_by_desc(entity::sys_log::Column::CreateTime).paginate(self.db, q.page_size as u64).fetch_page((q.page_num - 1) as u64).await?;
        Ok(PageData::new(rows.iter().map(|r| LogVO { id: r.id, module: r.module, action_type: r.action_type, title: r.title.clone(), content: r.content.clone(), operator_id: r.operator_id, operator_name: r.operator_name.clone(), request_uri: r.request_uri.clone(), request_method: r.request_method.clone(), ip: r.ip.clone(), province: r.province.clone(), city: r.city.clone(), device: r.device.clone(), os: r.os.clone(), browser: r.browser.clone(), status: r.status, error_msg: r.error_msg.clone(), execution_time: r.execution_time, create_time: r.create_time.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()) }).collect(), total))
    }

    /// 访问趋势
    pub async fn get_visit_trend(&self) -> AppResult<serde_json::Value> { Ok(serde_json::json!([])) }

    /// 访问概览
    pub async fn get_visit_overview(&self) -> AppResult<serde_json::Value> { Ok(serde_json::json!({"totalVisits":0,"todayVisits":0})) }
}
