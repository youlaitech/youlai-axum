use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn swagger_ui() -> SwaggerUi {
    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "youlai-rust API",
            version = "1.0.0",
            description = "Rust Axum SeaORM PostgreSQL admin backend"
        ),
        tags(
            (name = "auth", description = "login / logout / token / captcha"),
            (name = "sys:user", description = "user CRUD"),
            (name = "sys:role", description = "role CRUD"),
            (name = "sys:menu", description = "menu / routes"),
            (name = "sys:dept", description = "dept tree CRUD"),
            (name = "sys:dict", description = "dict type / dict items"),
            (name = "sys:config", description = "system config"),
            (name = "sys:notice", description = "notice management"),
            (name = "sys:log", description = "operation log"),
        ),
    )]
    struct ApiDoc;

    SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi())
}
