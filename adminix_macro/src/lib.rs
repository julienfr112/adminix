use proc_macro::TokenStream;
use quote::{format_ident, quote};
use sqlx::sqlite::SqliteConnection;
use sqlx::Connection;
use syn::Ident;

fn cb_ident(table: &String, method: &str) -> Ident {
    format_ident!("__adminix_{}_{}", &method, &table)
}

#[derive(sqlx::FromRow, Debug)]
struct Col {
    name: String,
    coltype: String,
    notnull: i32,
    fk: Option<String>,
}

#[derive(Debug)]
struct Table {
    name: String,
    columns: Vec<Col>,
}

#[derive(sqlx::FromRow, Debug)]
struct DBTable {
    name: String,
}

async fn tables() -> Vec<Table> {
    let mut res: Vec<Table> = vec![];
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL env not set");
    //println!("{}", std::env::current_dir().unwrap().display());
    let mut conn = SqliteConnection::connect(&database_url)
        .await
        .expect(&format!("cannot connect to db : {}",&database_url));
    let tables: Vec<DBTable> =
        sqlx::query_as::<_, DBTable>("select name from sqlite_master where type='table'")
            .fetch_all(&mut conn)
            .await
            .expect("Cannot get tables from sqlite");

    for dbtable in tables {
        let table_name = dbtable.name;
        if !table_name.starts_with("sqlite_") && !table_name.starts_with("__") {
            let cols: Vec<Col> = sqlx::query_as::<_,Col>(r#"
                SELECT 
                    cols.name as name, cols.type as coltype, cols.`notnull` as `notnull`, fk.`table` as `fk`
                        FROM pragma_table_info(?1) as cols 
                    left join 
                        pragma_foreign_key_list(?1) as fk 
                    on fk.`from` = cols.name
                    where cols.name != 'id'
            "#).bind(table_name.clone()).fetch_all(&mut conn).await.expect("cannot fetch columns");

            res.push(Table {
                name: table_name,
                columns: cols,
            });
        }
    }
    res
}

fn ctype(col: &Col) -> proc_macro2::TokenStream {
    if col.notnull == 1 {
        match col.coltype.as_str() {
            "INTEGER" => quote! { i64 },
            "TEXT" => quote! { String },
            "FLOAT" => quote! { f32 },
            "BLOB" => quote! { Vec<u8> },
            t => panic!("Unsupported db type : {}  in col {}", t, col.name),
        }
    } else {
        match col.coltype.as_str() {
            "INTEGER" => quote! { Option<i64> },
            "TEXT" => quote! { Option<String> },
            "FLOAT" => quote! { Option<f32> },
            "BLOB" => quote! { Option<Vec<u8>> },
            t => panic!("Unsupported supported db type : {}  in col {}", t, col.name),
        }
    }
}

#[proc_macro]
#[actix_web::main]
pub async fn prepare(_input: TokenStream) -> TokenStream {
    let tables = tables().await;
    let names = tables.iter().map(|t| t.name.clone()).collect::<Vec<_>>();

    let gets = tables
        .iter()
        .map(|t| cb_ident(&t.name, "get"))
        .collect::<Vec<_>>();

    let posts = tables
        .iter()
        .map(|t| cb_ident(&t.name, "post"))
        .collect::<Vec<_>>();

    let deletes = tables
        .iter()
        .map(|t| cb_ident(&t.name, "delete"))
        .collect::<Vec<_>>();

    let urls = tables
        .iter()
        .map(|t| format!("/admin/{}", t.name))
        .collect::<Vec<_>>();

    let mut code = quote! {
        #[derive(::sqlx::FromRow, ::serde::Deserialize)]
        pub struct Id {
            id: i64,
        }

        fn __adminix_base(body: ::maud::Markup) -> ::maud::Markup {
            ::maud::html! {
                (::maud::DOCTYPE)
                head {
                    link rel="stylesheet" href="/admin/style.css" {}
                }
                body {
                    div {
                        #(
                            a href=#urls { #names } " "
                        )*
                    }
                    (body)
                }
                script src="/admin/script.js" {}
            }
        }

        async fn __adminix_home() -> ::maud::Markup {
            __adminix_base(::maud::html! {"Welcome to admin"})
        }
    };

    let mut reverse_fk: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();

    for table in tables.iter() {
        for col in table.columns.iter() {
            if col.fk.is_some() {
                let k = col.fk.clone().unwrap();
                let v = (table.name.clone(), col.name.clone());
                if let Some(vs) = reverse_fk.get_mut(&k) {
                    vs.push(v);
                } else {
                    reverse_fk.insert(k, vec![v]);
                }
            }
        }
    }

    for table in tables {
        let tablename = table.name.clone();
        let istruct = format_ident!("__adminix_S{}", table.name);
        let scolnames = table
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        let inames: Vec<_> = scolnames.iter().map(|s| format_ident!("{}", s)).collect();
        let url = format!("/admin/{}", table.name);

        let coldbtypes = table
            .columns
            .iter()
            .map(|c| {
                if c.fk.is_some() {
                    "FK".to_string()
                } else {
                    c.coltype.clone()
                }
            })
            .collect::<Vec<_>>();
        let coltypes = table.columns.iter().map(|c| ctype(&c)).collect::<Vec<_>>();

        let inputs = table
            .columns
            .iter()
            .map(|c| {
                let colname = c.name.clone();
                let n = format_ident!("{}", colname);
                match (c.fk.clone(), c.notnull, c.coltype.as_str()) {
                    (Some(fk_table), _, _) => {
                        quote! { 
                            a href={"" #fk_table "#id" (value.#n)} {(value.#n)} 
                            input type="hidden" name=#colname value=(value.#n);
                        }
                    }
                    (None, _, "BLOB") => quote! { img src="mybase64image" {} },
                    (None, 1, _t) => {
                        quote! { input type="text" name=#colname value=(value.#n) size="10"; }
                    }
                    (None, 0, _t) => quote! {
                        @if value.#n.is_some() {
                            input type="text" name=#colname value=(value.#n.unwrap()) size="10";
                        }
                        @else {
                            input type="text" name=#colname value="_AX_NULL" size="10";
                        }
                    },
                    t => panic!(
                        "Unemplemented html converstion from (fk, notnull, type)= {:?}",
                        t
                    ),
                }
            })
            .collect::<Vec<_>>();

        let reverse_links = if let Some(v) = reverse_fk.get(&tablename) {
            let vquote = v.iter().map(|(rtable, rcol)| {
                quote! { a href={"/admin/" #rtable "?" #rcol "=" (value.id)} { #rtable} " " }
                //quote! { #rtable }
            });
            quote! { 
                "reverse"
                #(#vquote)* 
            }
        } else {
            quote! {}
        };

        code.extend(quote! {
            #[derive(::sqlx::FromRow, ::serde::Deserialize)]
            pub struct #istruct {
                id: i64,
                #(#inames: #coltypes),*
            }
        });

        let get = cb_ident(&table.name, "get");
        let sqlget = format!("SELECT id,{} FROM {}", scolnames.join(","), &tablename);
        code.extend(quote! {
            async fn #get(
                pool: ::actix_web::web::Data<sqlx::SqlitePool>,
                filters: ::actix_web::web::Query<::std::collections::HashMap<String,String>>,
            ) -> ::actix_web::Result<::maud::Markup> {
                let kv = filters.iter().next();
                let scolnames = vec!(#(#scolnames),*);
                let mut sql = #sqlget.to_string();
                if kv.is_some() {
                    let (k,v)=kv.unwrap();
                    if k=="id" || scolnames.iter().any(|c| c==k) {
                        sql.push_str(&format!(" where {}={}",k,v));  // SQL INJECTION !!!
                    }
                }
                let values: Vec<#istruct> = ::sqlx::query_as(&sql)
                    .fetch_all(pool.get_ref())
                    .await
                    .map_err(|e| ::actix_web::error::ErrorInternalServerError(e.to_string()))?;
                Ok( __adminix_base( ::maud::html! {
                    h2 {
                        "Table " #tablename " "
                        small {
                            a href={"/admin/" #tablename} { "🗘" }
                        }
                    }
                    table .sortable {
                        thead {
                            tr {
                                th { "id" }
                                #(
                                    th {
                                        #scolnames
                                        br;
                                        "(" #coldbtypes ")" 
                                    }
                                )*
                                th {}
                                th {}
                            }
                            tr {
                                th { form {
                                    input name="id" size="5";
                                }}
                                #(
                                    th { //#scolnames "(" #coldbtypes ")" 
                                        form {
                                            input name=#scolnames size="10";
                                        }
                                    }
                                )*
                                th {}
                                th {}
                            }
                        }
                        tbody {
                            @for value in values {
                                tr id={"id" (value.id)} {
                                    form style="display:inline-block" method="post" action={"/admin/" #tablename} 
                                    {
                                        td.rowid {
                                            label {
                                                (value.id)
                                                input type="hidden" name="id" value=(value.id);
                                            }
                                            .tooltip {                                                
                                                #reverse_links
                                            }
                                        }
                                        #(
                                            td { #inputs }
                                        )*
                                        td { input type="submit" value="🖫"; }
                                    }
                                    form style="display:inline-block" method="post" action={"/admin/" #tablename "/delete"} {
                                        input type="hidden" name="id" value=(value.id);
                                        td { input type="submit" value="🗑"; }
                                    }
                                }
                            }
                        }
                        tfoot {
                            tr {
                                form method="post" action={"/admin/" #tablename } {
                                    td {
                                        label {
                                            "∅"
                                            input type="hidden" name="id" value="-1";
                                        }
                                    }
                                    #(
                                        td { input type="text" name=#scolnames placeholder=#scolnames size="10";}
                                    )*
                                    td { input type="submit" value="🖫"; }
                                    td {}
                                }
                            }
                        }
                    }
                }))
            }
        });

        let post = cb_ident(&table.name, "post");
        let sqlinsert = format!(
            "insert into {}({}) values ({})",
            table.name,
            scolnames.join(","),
            std::iter::repeat("?")
                .take(scolnames.len())
                .collect::<Vec<_>>()
                .join(",")
        );
        let sqlupdate = format!(
            "update {} set {} where id=?",
            table.name,
            scolnames
                .iter()
                .map(|c| format!("{}=?", c))
                .collect::<Vec<_>>()
                .join(",")
        );
        let sqldelete = format!("delete from {} where id=?", table.name);
        code.extend(quote! {
            async fn #post(
                req: ::actix_web::HttpRequest,
                pool: ::actix_web::web::Data<sqlx::SqlitePool>,
                form: ::actix_web::web::Form<#istruct>,
            ) -> ::actix_web::HttpResponse {
                let item = form.into_inner();
                if item.id == -1 {
                    sqlx::query!(#sqlinsert, #(item.#inames),*)
                        .execute(pool.get_ref())
                        .await.unwrap();
                } else {
                    ::sqlx::query!(#sqlupdate,  #(item.#inames),* ,item.id)
                        .execute(pool.get_ref())
                        .await.unwrap();
               }
                ::actix_web::HttpResponse::SeeOther()
                    .header(::actix_web::http::header::LOCATION, #url)
                    .finish()
            }
        });

        let delete = cb_ident(&table.name, "delete");
        code.extend(quote! {
            async fn #delete(
                req: ::actix_web::HttpRequest,
                pool: ::actix_web::web::Data<sqlx::SqlitePool>,
                form: ::actix_web::web::Form<Id>,
            ) -> ::actix_web::HttpResponse {
                let id = form.into_inner();
                ::sqlx::query!(#sqldelete, id.id)
                    .execute(pool.get_ref())
                    .await.unwrap();
                ::actix_web::HttpResponse::SeeOther()
                    .header(::actix_web::http::header::LOCATION, #url)
                    .finish()
            }
        });
    }

    let scriptjs = include_str!("script.js");
    let stylecss = include_str!("style.css");

    code.extend(quote! {
        fn configure_adminix(cfg: &mut ::actix_web::web::ServiceConfig) {
            cfg.service(
                ::actix_web::web::resource("")
                .route(::actix_web::web::get().to(__adminix_home))
            )
            .service(
                ::actix_web::web::resource("script.js")
                .route(::actix_web::web::get().to(|| ::actix_web::web::HttpResponse::Ok().body(#scriptjs)))
            )
            .service(
                ::actix_web::web::resource("style.css")
                .route(::actix_web::web::get().to(|| ::actix_web::web::HttpResponse::Ok().body(#stylecss)))
            )
            #(
                .service(
                    ::actix_web::web::scope(#names)
                    .service(
                        ::actix_web::web::resource("")
                            .route(::actix_web::web::get().to(#gets))
                            .route(::actix_web::web::post().to(#posts)),
                    )
                    .service(
                       ::actix_web::web::resource("/delete").route(::actix_web::web::post().to(#deletes)),
                    ),
                )
            )*
            ;
        }
    });

    code.into()
}
