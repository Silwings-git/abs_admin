use crate::domain::dto::{RoleAddDTO, RoleEditDTO, RolePageDTO};
use crate::domain::table::{SysRole, SysRolePermission, SysUserRole};
use crate::domain::vo::{SysPermissionVO, SysRoleVO};
use crate::error::Result;
use crate::pool;
use crate::context::CONTEXT;
use rbatis::{Page, PageRequest};
use std::collections::{BTreeMap, HashMap};

const RES_KEY: &'static str = "sys_role:all";

///Role of service
pub struct SysRoleService {}

impl SysRoleService {
    pub async fn page(&self, arg: &RolePageDTO) -> Result<Page<SysRoleVO>> {
        let data = SysRole::select_page_by_name(
            pool!(),
            &PageRequest::from(arg),
            arg.name.as_deref().unwrap_or_default(),
        )
            .await?;
        let all_role = self.finds_all_map().await?;
        let mut page = Page::<SysRoleVO>::from(data);
        for mut vo in &mut page.records {
            self.loop_find_childs(&mut vo, &all_role);
        }
        Ok(page)
    }

    /// Role-level data
    pub async fn finds_layer(&self) -> Result<Vec<SysRoleVO>> {
        let all_roles = self.finds_all_map().await?;
        let mut data = Vec::with_capacity(all_roles.len());
        for (_k, v) in &all_roles {
            if v.parent_id.is_none() {
                let mut top = SysRoleVO::from(v.clone());
                self.loop_find_childs(&mut top, &all_roles);
                data.push(top);
            }
        }
        Ok(data)
    }

    pub async fn finds_all(&self) -> Result<Vec<SysRole>> {
        let js = CONTEXT
            .cache_service
            .get_json::<Option<Vec<SysRole>>>(RES_KEY)
            .await;
        let is_empty = match js {
            Err(_) => true,
            Ok(Some(ref inner)) => inner.is_empty(),
            Ok(None) => true,
        };
        if is_empty
        {
            let all = self.update_cache().await?;
            return Ok(all);
        }
        if CONTEXT.config.debug {
            log::info!("[abs_admin] get from cache:{}", RES_KEY);
        }
        Ok(js?.unwrap_or_default())
    }

    pub async fn update_cache(&self) -> Result<Vec<SysRole>> {
        let all = SysRole::select_all(pool!()).await?;
        CONTEXT.cache_service.set_json(RES_KEY, &all).await?;
        Ok(all)
    }

    /// All user ids - User Map data
    pub async fn finds_all_map(&self) -> Result<HashMap<String, SysRole>> {
        let all = self.finds_all().await?;
        let mut result = HashMap::with_capacity(all.capacity());
        for x in all {
            result.insert(x.id.as_deref().unwrap_or_default().to_string(), x);
        }
        Ok(result)
    }

    pub async fn add(&self, arg: RoleAddDTO) -> Result<(u64, String)> {
        let role = SysRole::from(arg);
        let result = (
            SysRole::insert(pool!(), &role).await?.rows_affected,
            role.id.clone().unwrap_or_default(),
        );
        self.update_cache().await?;
        Ok(result)
    }

    pub async fn edit(&self, arg: RoleEditDTO) -> Result<u64> {
        let role = SysRole::from(arg);
        let result = SysRole::update_by_column(pool!(), &role, "id").await;
        self.update_cache().await?;
        Ok(result?.rows_affected)
    }

    pub async fn remove(&self, id: &str) -> Result<u64> {
        let result = SysRole::delete_by_column(pool!(), "id", id).await?;
        self.update_cache().await?;
        Ok(result.rows_affected)
    }

    pub async fn finds(&self, ids: &Vec<String>) -> Result<Vec<SysRole>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(SysRole::select_in_column(pool!(), "id", ids).await?)
    }

    pub async fn find_role_res(&self, role_ids: &Vec<String>) -> Result<Vec<SysRolePermission>> {
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(SysRolePermission::select_in_column(pool!(), "role_id", role_ids).await?)
    }

    pub async fn find_user_permission(
        &self,
        user_id: &str,
        all_res: &BTreeMap<String, SysPermissionVO>,
    ) -> Result<Vec<String>> {
        let user_roles = SysUserRole::select_by_column(pool!(), "user_id", user_id).await?;
        let role_ids = rbatis::table_field_vec!(&user_roles, role_id).iter().map(|v| v.to_string()).collect();
        let role_res = self
            .find_role_res(&role_ids)
            .await?;
        let res = CONTEXT
            .sys_permission_service
            .finds_layer(
                &rbatis::table_field_vec!(role_res, permission_id),
                &all_res,
            )
            .await?;
        let permissions = rbatis::table_field_vec!(res, permission);
        Ok(permissions)
    }

    ///Loop to find the parent-child associative relation array
    pub fn loop_find_childs(&self, arg: &mut SysRoleVO, roles: &HashMap<String, SysRole>) {
        let mut childs = Vec::with_capacity(roles.len());
        for (_key, x) in roles {
            if x.parent_id.is_some() && x.parent_id.eq(&arg.id) {
                let mut item = SysRoleVO::from(x.clone());
                self.loop_find_childs(&mut item, roles);
                childs.push(item);
            }
        }
        if !childs.is_empty() {
            arg.childs = Some(childs);
        }
    }
}
