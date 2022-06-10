#version 430 core

in block {
    vec3 v3Color;
} In;

layout (location = 0) out vec4 Out_v4Color;

void main()
{
    Out_v4Color = vec4(In.v3Color, 1.0f);
}