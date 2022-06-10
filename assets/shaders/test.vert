#version 430 core

#extension GL_ARB_shader_storage_buffer_object : require

layout (std140, binding = 0) buffer CB0
{
    mat4 Transforms[];
};

uniform mat4 View;
uniform mat4 Projection;

layout (location = 0) in vec3 In_v3Pos;
layout (location = 1) in vec3 In_v3Color;
layout (location = 2) in uint In_iDrawID;

out block {
    vec3 v3Color;
} Out;

void main()
{
    mat4 World = Transforms[In_iDrawID];
    vec3 worldPos = vec3(World * vec4(In_v3Pos, 1));
    gl_Position = Projection * View * vec4(worldPos, 1);
    
    Out.v3Color = In_v3Color;
}