/*
 * Stub implementations of nginx C symbols for unit test linking.
 * These are never invoked at runtime - they satisfy the linker for test builds.
 */
#include <stddef.h>
#include <stdlib.h>

typedef unsigned long ngx_uint_t;
typedef long ngx_int_t;
typedef unsigned char u_char;

/* Core stubs */
void *ngx_palloc(void *pool, size_t size) { return malloc(size); }
void *ngx_pnalloc(void *pool, size_t size) { return malloc(size); }
void *ngx_pcalloc(void *pool, size_t size) { return calloc(1, size); }
void *ngx_alloc(size_t size, void *log) { return malloc(size); }
void *ngx_calloc(size_t size, void *log) { return calloc(1, size); }
ngx_uint_t ngx_hash_strlow(u_char *dst, u_char *src, size_t n) { return 0; }
void *ngx_pool_cleanup_add(void *p, size_t size) { return NULL; }

/* List / Array stubs */
void *ngx_list_push(void *l) { return NULL; }
void *ngx_array_push(void *a) { return calloc(1, 256); }

/* HTTP stubs */
ngx_int_t ngx_http_send_header(void *r) { return -1; }
ngx_int_t ngx_http_output_filter(void *r, void *in) { return -1; }
void *ngx_create_temp_buf(void *pool, size_t size) { return NULL; }
void *ngx_alloc_chain_link(void *pool) { return NULL; }
ngx_int_t ngx_http_complex_value(void *r, void *val, void *str) { return -1; }
ngx_int_t ngx_http_discard_request_body(void *r) { return 0; }
ngx_int_t ngx_http_subrequest(void *r, void *uri, void *args, void **psr, void *ps, int flags) { return -1; }
void ngx_http_named_location(void *r, void *name) {}
void ngx_http_internal_redirect(void *r, void *uri, void *args) {}

/* Global modules referenced by ngx crate */
char ngx_http_core_module[4096];
