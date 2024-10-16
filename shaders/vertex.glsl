#version 460

layout(location = 0) in vec2 positions;
layout(location = 1) in vec3 colors;

layout(location = 0) out vec3 color;

void main(){
    color = colors;
    gl_Position = vec4(positions, 0.0, 1.0);
}