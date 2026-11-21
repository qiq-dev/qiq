@.fmt.int = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@.fmt.bool = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@.fmt.double = private unnamed_addr constant [4 x i8] c"%g\0A\00", align 1
@.fmt.string = private unnamed_addr constant [4 x i8] c"%s\0A\00", align 1
@.str.null = private unnamed_addr constant [5 x i8] c"null\00", align 1
@.str0 = private unnamed_addr constant [11 x i8] c"page label\00", align 1
@.str1 = private unnamed_addr constant [14 x i8] c"helpers label\00", align 1
@.str2 = private unnamed_addr constant [6 x i8] c"title\00", align 1
@.str3 = private unnamed_addr constant [8 x i8] c"Welcome\00", align 1
@.str4 = private unnamed_addr constant [17 x i8] c"<h1>Welcome</h1>\00", align 1
@.str5 = private unnamed_addr constant [13 x i8] c"<p>shown</p>\00", align 1
@.str6 = private unnamed_addr constant [5 x i8] c"Home\00", align 1
@.class.name0 = private unnamed_addr constant [10 x i8] c"Exception\00", align 1
@.class.name1 = private unnamed_addr constant [17 x i8] c"RuntimeException\00", align 1
@.class.name2 = private unnamed_addr constant [12 x i8] c"IOException\00", align 1
@.class.name3 = private unnamed_addr constant [22 x i8] c"FileNotFoundException\00", align 1
@.class.name4 = private unnamed_addr constant [26 x i8] c"PermissionDeniedException\00", align 1
@.class.name5 = private unnamed_addr constant [21 x i8] c"KeyNotFoundException\00", align 1
@.class.name6 = private unnamed_addr constant [20 x i8] c"OutOfRangeException\00", align 1
@.class.name7 = private unnamed_addr constant [21 x i8] c"InvalidUtf8Exception\00", align 1
@.class.name8 = private unnamed_addr constant [20 x i8] c"ArithmeticException\00", align 1
@.class.name9 = private unnamed_addr constant [24 x i8] c"DivisionByZeroException\00", align 1
@.class.name10 = private unnamed_addr constant [25 x i8] c"InvalidArgumentException\00", align 1
@.class.name11 = private unnamed_addr constant [9 x i8] c"Template\00", align 1
@.class.name12 = private unnamed_addr constant [9 x i8] c"BaseView\00", align 1
@.class.name13 = private unnamed_addr constant [5 x i8] c"Page\00", align 1
@qiq_class_names = unnamed_addr constant [14 x ptr] [ptr @.class.name0, ptr @.class.name1, ptr @.class.name2, ptr @.class.name3, ptr @.class.name4, ptr @.class.name5, ptr @.class.name6, ptr @.class.name7, ptr @.class.name8, ptr @.class.name9, ptr @.class.name10, ptr @.class.name11, ptr @.class.name12, ptr @.class.name13], align 8
@qiq_pending_exception = external global ptr
@qiq_dispatch_template.1.Template_setVariable = private unnamed_addr constant [14 x ptr] [ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr @qiq_page.2.Page_setVariable], align 8
@qiq_dispatch_template.1.Template_getHtml = private unnamed_addr constant [14 x ptr] [ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr null, ptr @qiq_page.2.Page_getHtml], align 8

declare i32 @printf(ptr, ...)
declare i32 @strcmp(ptr, ptr)
declare ptr @calloc(i64, i64)
declare ptr @qiq_map_new(i32)
declare void @qiq_map_set(ptr, i64, i64)
declare i64 @qiq_map_get(ptr, i64)
declare i64 @qiq_map_len(ptr)
declare i64 @qiq_map_key_at(ptr, i64)
declare i64 @qiq_map_value_at(ptr, i64)
declare i64 @qiq_string_len(ptr)
declare ptr @qiq_string_char_at(ptr, i64)
declare ptr @qiq_string_concat(ptr, ptr)
declare i64 @qiq_int_divide(i64, i64)
declare i64 @qiq_int_modulo(i64, i64)
declare i64 @qiq_int_power(i64, i64)
declare i64 @qiq_int_shift_left(i64, i64)
declare i64 @qiq_int_shift_right(i64, i64)
declare double @llvm.pow.f64(double, double)
declare void @qiq_uncaught(ptr, ptr)
declare ptr @qiq_array_new(i64, i64)
declare i64 @qiq_array_length(ptr)
declare void @qiq_array_check_index(ptr, i64)
declare void @qiq_print_value(ptr, i64)
declare void @qiq_print_space()
declare void @qiq_print_newline()
declare i64 @qiq_double_to_int(double)
declare zeroext i8 @qiq_int_to_byte(i64)
declare i64 @qiq_string_to_int(ptr)
declare double @qiq_string_to_double(ptr)
declare ptr @qiq_int_to_string(i64)
declare ptr @qiq_double_to_string(double)

define ptr @qiq.page.2.label() {
entry:
  ret ptr getelementptr inbounds ([11 x i8], ptr @.str0, i32 0, i32 0)
unwind:
  ret ptr null
}

define ptr @qiq.helpers.3.label() {
entry:
  ret ptr getelementptr inbounds ([14 x i8], ptr @.str1, i32 0, i32 0)
unwind:
  ret ptr null
}

define void @qiq.helpers.3.render(ptr %arg0) {
entry:
  %v0 = load i64, ptr %arg0, align 8
  %v1 = getelementptr inbounds ptr, ptr @qiq_dispatch_template.1.Template_setVariable, i64 %v0
  %v2 = load ptr, ptr %v1, align 8
  call void %v2(ptr %arg0, ptr getelementptr inbounds ([6 x i8], ptr @.str2, i32 0, i32 0), ptr getelementptr inbounds ([8 x i8], ptr @.str3, i32 0, i32 0))
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  %v3 = load i64, ptr %arg0, align 8
  %v4 = getelementptr inbounds ptr, ptr @qiq_dispatch_template.1.Template_getHtml, i64 %v3
  %v5 = load ptr, ptr %v4, align 8
  %v6 = call ptr %v5(ptr %arg0, ptr getelementptr inbounds ([17 x i8], ptr @.str4, i32 0, i32 0))
  %pending1 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown1 = icmp ne ptr %pending1, null
  br i1 %thrown1, label %unwind, label %ok1
ok1:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v6)
  ret void
unwind:
  ret void
}

define void @show(ptr %arg0) {
entry:
  %v0 = load i64, ptr %arg0, align 8
  %v1 = getelementptr inbounds ptr, ptr @qiq_dispatch_template.1.Template_getHtml, i64 %v0
  %v2 = load ptr, ptr %v1, align 8
  %v3 = call ptr %v2(ptr %arg0, ptr getelementptr inbounds ([13 x i8], ptr @.str5, i32 0, i32 0))
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v3)
  ret void
unwind:
  ret void
}

define void @qiq_stdlib.exceptions.Exception___construct(ptr %this, ptr %arg0, i64 %arg1) {
entry:
  %v0 = alloca ptr, align 8
  %v1 = alloca i64, align 8
  store ptr %arg0, ptr %v0, align 8
  store i64 %arg1, ptr %v1, align 8
  %v2 = load ptr, ptr %v0, align 8
  %v3 = ptrtoint ptr %v2 to i64
  %v4 = getelementptr inbounds i64, ptr %this, i64 1
  store i64 %v3, ptr %v4, align 8
  %v5 = load i64, ptr %v1, align 8
  %v6 = add i64 %v5, 0
  %v7 = getelementptr inbounds i64, ptr %this, i64 2
  store i64 %v6, ptr %v7, align 8
  ret void
unwind:
  ret void
}

define ptr @qiq_stdlib.exceptions.Exception_getMessage(ptr %this) {
entry:
  %v0 = getelementptr inbounds i64, ptr %this, i64 1
  %v1 = load i64, ptr %v0, align 8
  %v2 = inttoptr i64 %v1 to ptr
  ret ptr %v2
unwind:
  ret ptr null
}

define i64 @qiq_stdlib.exceptions.Exception_getCode(ptr %this) {
entry:
  %v0 = getelementptr inbounds i64, ptr %this, i64 2
  %v1 = load i64, ptr %v0, align 8
  %v2 = add i64 %v1, 0
  ret i64 %v2
unwind:
  ret i64 0
}

define void @qiq_template.1.BaseView___construct(ptr %this, ptr %arg0) {
entry:
  %v0 = alloca ptr, align 8
  store ptr %arg0, ptr %v0, align 8
  %v1 = load ptr, ptr %v0, align 8
  %v2 = ptrtoint ptr %v1 to i64
  %v3 = getelementptr inbounds i64, ptr %this, i64 1
  store i64 %v2, ptr %v3, align 8
  ret void
unwind:
  ret void
}

define ptr @qiq_template.1.BaseView_getTitle(ptr %this) {
entry:
  %v0 = getelementptr inbounds i64, ptr %this, i64 1
  %v1 = load i64, ptr %v0, align 8
  %v2 = inttoptr i64 %v1 to ptr
  ret ptr %v2
unwind:
  ret ptr null
}

define void @qiq_page.2.Page___construct(ptr %this, ptr %arg0) {
entry:
  %v0 = alloca ptr, align 8
  store ptr %arg0, ptr %v0, align 8
  %v1 = load ptr, ptr %v0, align 8
  call void @qiq_template.1.BaseView___construct(ptr %this, ptr %v1)
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  ret void
unwind:
  ret void
}

define void @qiq_page.2.Page_setVariable(ptr %this, ptr %arg0, ptr %arg1) {
entry:
  %v0 = alloca ptr, align 8
  %v1 = alloca ptr, align 8
  store ptr %arg0, ptr %v0, align 8
  store ptr %arg1, ptr %v1, align 8
  %v2 = call ptr @qiq.page.2.label()
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v2)
  ret void
unwind:
  ret void
}

define ptr @qiq_page.2.Page_getHtml(ptr %this, ptr %arg0) {
entry:
  %v0 = alloca ptr, align 8
  store ptr %arg0, ptr %v0, align 8
  %v1 = load ptr, ptr %v0, align 8
  ret ptr %v1
unwind:
  ret ptr null
}

define void @qiq_raise(i32 %kind, ptr %message) {
entry:
  switch i32 %kind, label %kind0 [ i32 1, label %kind1 i32 2, label %kind2 i32 3, label %kind3 i32 4, label %kind4 i32 5, label %kind5 i32 6, label %kind6 i32 7, label %kind7 i32 8, label %kind8 i32 9, label %kind9 ]
kind0:
  %object0 = call ptr @calloc(i64 3, i64 8)
  store i64 1, ptr %object0, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object0, ptr %message, i64 0)
  store ptr %object0, ptr @qiq_pending_exception, align 8
  ret void
kind1:
  %object1 = call ptr @calloc(i64 3, i64 8)
  store i64 8, ptr %object1, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object1, ptr %message, i64 0)
  store ptr %object1, ptr @qiq_pending_exception, align 8
  ret void
kind2:
  %object2 = call ptr @calloc(i64 3, i64 8)
  store i64 9, ptr %object2, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object2, ptr %message, i64 0)
  store ptr %object2, ptr @qiq_pending_exception, align 8
  ret void
kind3:
  %object3 = call ptr @calloc(i64 3, i64 8)
  store i64 5, ptr %object3, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object3, ptr %message, i64 0)
  store ptr %object3, ptr @qiq_pending_exception, align 8
  ret void
kind4:
  %object4 = call ptr @calloc(i64 3, i64 8)
  store i64 7, ptr %object4, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object4, ptr %message, i64 0)
  store ptr %object4, ptr @qiq_pending_exception, align 8
  ret void
kind5:
  %object5 = call ptr @calloc(i64 3, i64 8)
  store i64 10, ptr %object5, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object5, ptr %message, i64 0)
  store ptr %object5, ptr @qiq_pending_exception, align 8
  ret void
kind6:
  %object6 = call ptr @calloc(i64 3, i64 8)
  store i64 2, ptr %object6, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object6, ptr %message, i64 0)
  store ptr %object6, ptr @qiq_pending_exception, align 8
  ret void
kind7:
  %object7 = call ptr @calloc(i64 3, i64 8)
  store i64 3, ptr %object7, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object7, ptr %message, i64 0)
  store ptr %object7, ptr @qiq_pending_exception, align 8
  ret void
kind8:
  %object8 = call ptr @calloc(i64 3, i64 8)
  store i64 4, ptr %object8, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object8, ptr %message, i64 0)
  store ptr %object8, ptr @qiq_pending_exception, align 8
  ret void
kind9:
  %object9 = call ptr @calloc(i64 3, i64 8)
  store i64 6, ptr %object9, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object9, ptr %message, i64 0)
  store ptr %object9, ptr @qiq_pending_exception, align 8
  ret void
}
define i32 @main() {
entry:
  %v0 = call ptr @calloc(i64 2, i64 8)
  store i64 13, ptr %v0, align 8
  call void @qiq_page.2.Page___construct(ptr %v0, ptr getelementptr inbounds ([5 x i8], ptr @.str6, i32 0, i32 0))
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  %v1 = call ptr @qiq_template.1.BaseView_getTitle(ptr %v0)
  %pending1 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown1 = icmp ne ptr %pending1, null
  br i1 %thrown1, label %unwind, label %ok1
ok1:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v1)
  call void @qiq.helpers.3.render(ptr %v0)
  %pending2 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown2 = icmp ne ptr %pending2, null
  br i1 %thrown2, label %unwind, label %ok2
ok2:
  %v2 = call ptr @qiq.helpers.3.label()
  %pending3 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown3 = icmp ne ptr %pending3, null
  br i1 %thrown3, label %unwind, label %ok3
ok3:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v2)
  call void @show(ptr %v0)
  %pending4 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown4 = icmp ne ptr %pending4, null
  br i1 %thrown4, label %unwind, label %ok4
ok4:
  ret i32 0
unwind:
  %uncaught = load ptr, ptr @qiq_pending_exception, align 8
  %uncaught.class = load i64, ptr %uncaught, align 8
  %uncaught.slot = getelementptr inbounds ptr, ptr @qiq_class_names, i64 %uncaught.class
  %uncaught.name = load ptr, ptr %uncaught.slot, align 8
  %uncaught.message = call ptr @qiq_stdlib.exceptions.Exception_getMessage(ptr %uncaught)
  call void @qiq_uncaught(ptr %uncaught.name, ptr %uncaught.message)
  unreachable
}
