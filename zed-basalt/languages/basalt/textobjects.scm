(function_definition
  body: (block
    "{" @function.inside
    "}" @function.inside)) @function.around

(method_definition
  body: (block
    "{" @function.inside
    "}" @function.inside)) @function.around

(type_definition) @class.around

(comment)+ @comment.around
