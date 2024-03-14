extern crate base64;
use base64::prelude::*;

use super::helpers::{decode_function, fetch_function};

use crate::{
    api::functions::types::{Stage, TestParam},
    db::{
        self,
        models::Function,
        schema::functions::{dsl, dsl::functions, function_name},
    },
    validation_functions,
};
use actix_web::{
    delete,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get, patch, post, put,
    web::{self, Json, Path},
    HttpResponse, Result, Scope,
};
use chrono::Utc;
use dashboard_auth::types::User;
use diesel::{delete, ExpressionMethods, QueryDsl, RunQueryDsl};
use serde_json::{json, Value};
use service_utils::service::types::DbConnection;
use validation_functions::{compile_fn, execute_fn};

use super::types::{CreateFunctionRequest, UpdateFunctionRequest};

pub fn endpoints() -> Scope {
    Scope::new("")
        .service(create)
        .service(update)
        .service(get)
        .service(list_functions)
        .service(delete_function)
        .service(test)
        .service(publish)
}

#[post("")]
async fn create(
    user: User,
    request: web::Json<CreateFunctionRequest>,
    db_conn: DbConnection,
) -> Result<Json<Function>> {
    let DbConnection(mut conn) = db_conn;
    let req = request.into_inner();

    if let Err(e) = compile_fn(&req.function, &req.function_name) {
        return Err(ErrorBadRequest(json!({ "message": e })));
    }

    let function = Function {
        function_name: req.function_name,
        draft_code: BASE64_STANDARD.encode(req.function),
        draft_runtime_version: req.runtime_version,
        draft_edited_by: user.email,
        draft_edited_at: Utc::now().naive_utc(),
        published_code: None,
        published_at: None,
        published_by: None,
        published_runtime_version: None,
        function_description: req.description,
    };

    let insert: Result<Function, diesel::result::Error> = diesel::insert_into(functions)
        .values(&function)
        .get_result(&mut conn);

    match insert {
        Ok(mut res) => {
            decode_function(&mut res)?;
            Ok(Json(res))
        }
        Err(e) => match e {
            diesel::result::Error::DatabaseError(kind, e) => {
                log::error!("Function error: {:?}", e);
                match kind {
                    diesel::result::DatabaseErrorKind::UniqueViolation => {
                        return Err(ErrorBadRequest(
                            json!({"message": "Function already exists."}),
                        ))
                    }
                    _ => {
                        return Err(ErrorBadRequest(
                            json!({"message": "An error occured please contact the admin"}),
                        ))
                    }
                }
            }
            _ => {
                log::error!("Function creation failed with error: {e}");
                return Err(ErrorInternalServerError(
                    json!({"message": "An error occured please contact the admin."}),
                ));
            }
        },
    }
}

#[patch("/{function_name}")]
async fn update(
    user: User,
    params: web::Path<String>,
    request: web::Json<UpdateFunctionRequest>,
    db_conn: DbConnection,
) -> Result<Json<Function>> {
    let DbConnection(mut conn) = db_conn;
    let req = request.into_inner();
    let f_name = params.into_inner();

    let result = match fetch_function(&f_name, &mut conn) {
        Ok(val) => val,
        Err(diesel::result::Error::NotFound) => {
            log::error!("Function not found.");
            return Err(ErrorBadRequest(json!({"message": "Function not found."})));
        }
        Err(e) => {
            log::error!("Failed to update Function with error: {e}");
            return Err(ErrorInternalServerError(
                json!({"message": "Failed to update Function"}),
            ));
        }
    };

    // Function Linter Check
    if let Some(function) = &req.function {
        if let Err(e) = compile_fn(function, &f_name) {
            return Err(ErrorBadRequest(json!({ "message": e })));
        }
    }

    let new_function = Function {
        function_name: f_name.to_owned(),
        draft_code: req.function.map_or_else(
            || result.draft_code.clone(),
            |func| BASE64_STANDARD.encode(func),
        ),
        draft_runtime_version: req
            .runtime_version
            .unwrap_or(result.draft_runtime_version),
        function_description: req.description.unwrap_or(result.function_description),
        draft_edited_by: user.email,
        draft_edited_at: Utc::now().naive_utc(),
        published_code: result.published_code,
        published_at: result.published_at,
        published_by: result.published_by,
        published_runtime_version: result.published_runtime_version,
    };

    let update: Result<Function, diesel::result::Error> = diesel::update(functions)
        .filter(db::schema::functions::function_name.eq(f_name))
        .set(new_function)
        .get_result(&mut conn);

    match update {
        Ok(mut res) => {
            decode_function(&mut res)?;
            Ok(Json(res))
        }
        Err(e) => {
            log::error!("Function updation failed with error: {e}");
            Err(ErrorInternalServerError(
                json!({"message": "Failed to update Function"}),
            ))
        }
    }
}

#[get("/{function_name}")]
async fn get(params: web::Path<String>, db_conn: DbConnection) -> Result<HttpResponse> {
    let DbConnection(mut conn) = db_conn;
    let f_name = params.into_inner();
    let result = fetch_function(&f_name, &mut conn);

    match result {
        Ok(mut function) => {
            decode_function(&mut function)?;
            Ok(HttpResponse::Ok().json(function))
        }
        Err(e) => {
            log::error!("Error getting function: {e}");
            Err(ErrorInternalServerError(
                json!({"message": "Function does not exists."}),
            ))
        }
    }
}

#[get("")]
async fn list_functions(db_conn: DbConnection) -> Result<Json<Vec<Function>>> {
    let DbConnection(mut conn) = db_conn;
    let result: Result<Vec<Function>, diesel::result::Error> =
        functions.get_results(&mut conn);

    match result {
        Ok(mut function_list) => {
            for function in function_list.iter_mut() {
                decode_function(function)?;
            }
            Ok(Json(function_list))
        }
        Err(e) => {
            log::error!("Error getting the functions: {e}");
            Err(ErrorInternalServerError(
                json!({"message": "Error getting the functions."}),
            ))
        }
    }
}

#[delete("/{function_name}")]
async fn delete_function(
    user: User,
    params: web::Path<String>,
    db_conn: DbConnection,
) -> Result<HttpResponse> {
    let DbConnection(mut conn) = db_conn;
    let f_name = params.into_inner();

    let deleted_row =
        delete(functions.filter(function_name.eq(&f_name))).execute(&mut conn);
    match deleted_row {
        Ok(0) => Err(ErrorNotFound(json!({"message": "Function not found."}))),
        Ok(_) => {
            log::info!("{f_name} function deleted by {}", user.email);
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            log::error!("function delete query failed with error: {e}");
            Err(ErrorInternalServerError(""))
        }
    }
}

#[put("/{function_name}/{stage}/test")]
async fn test(
    params: Path<TestParam>,
    request: web::Json<Value>,
    db_conn: DbConnection,
) -> actix_web::Result<HttpResponse> {
    let DbConnection(mut conn) = db_conn;
    let path_params = params.into_inner();
    let fun_name = &path_params.function_name;
    let req = request.into_inner();
    let mut function = match fetch_function(fun_name, &mut conn) {
        Ok(val) => val,
        Err(diesel::result::Error::NotFound) => {
            log::error!("Function not found.");
            return Err(ErrorBadRequest(json!({"message": "Function not found."})));
        }
        Err(e) => {
            log::error!("Failed to update Function with error: {e}");
            return Err(ErrorInternalServerError(
                json!({"message": "Failed to update Function due to unexpected DB issue"}),
            ));
        }
    };
    decode_function(&mut function)?;
    let result = match path_params.stage {
        Stage::DRAFT => execute_fn(&function.draft_code, fun_name, req),
        Stage::PUBLISHED => match function.published_code {
            Some(code) => execute_fn(&code, fun_name, req),
            None => {
                log::error!("Function test failed: function not published yet");
                Err((
                    "Function test failed as function not published yet".to_owned(),
                    None,
                ))
            }
        },
    };

    match result {
        Ok(stdout) => Ok(HttpResponse::Ok()
            .json(json!({"message": "Function validated the given value successfully", "stdout": stdout}))),
        Err((e, stdout)) => Err(ErrorBadRequest(json!({ "message": format!( "Function validation failed with error: {e}" ), "stdout": stdout }))),
    }
}

#[put("/{function_name}/publish")]
async fn publish(
    user: User,
    params: web::Path<String>,
    db_conn: DbConnection,
) -> actix_web::Result<HttpResponse> {
    let DbConnection(mut conn) = db_conn;
    let fun_name = params.into_inner();

    let function = match fetch_function(&fun_name, &mut conn) {
        Ok(val) => val,
        Err(diesel::result::Error::NotFound) => {
            log::error!("Function not found.");
            return Err(ErrorBadRequest(json!({"message": "Function not found."})));
        }
        Err(e) => {
            log::error!("Failed to update Function with error: {e}");
            return Err(ErrorInternalServerError(
                json!({"message": "Failed to update Function"}),
            ));
        }
    };

    let updated_function: Result<Function, diesel::result::Error> =
        diesel::update(functions)
            .filter(dsl::function_name.eq(fun_name))
            .set((
                dsl::published_code.eq(Some(function.draft_code.clone())),
                dsl::published_runtime_version
                    .eq(Some(function.draft_runtime_version.clone())),
                dsl::published_by.eq(Some(user.email)),
                dsl::published_at.eq(Some(Utc::now().naive_utc())),
            ))
            .get_result(&mut conn);

    match updated_function {
        Ok(_) => Ok(HttpResponse::Ok().json(json!({
            "message": "Function published successfully."
        }))),
        Err(e) => {
            log::error!("Function publish failed with error: {e}");
            Err(ErrorInternalServerError(
                json!({"message": "Failed to publish Function due to unexpected DB issue"}),
            ))
        }
    }
}
