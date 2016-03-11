use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

pub fn new<'a, 'b:'a>(gl: &'a GLContext) -> T<'b> {
  let components = vec!(
    (gl::VERTEX_SHADER, "
      #version 330 core

      void main() {
        if (gl_VertexID == 0) {
          gl_Position = vec4(1, -1, 0, 1);
        } else if (gl_VertexID == 1) {
          gl_Position = vec4(1, 1, 0, 1);
        } else if (gl_VertexID == 2) {
          gl_Position = vec4(-1, -1, 0, 1);
        } else if (gl_VertexID == 3) {
          gl_Position = vec4(-1, 1, 0, 1);
        }
      }".to_owned()),
    (gl::FRAGMENT_SHADER,
      format!(r#"
        #version 330 core

        const float CLOUD_HEIGHT = 1000;

        uniform vec2 window_size;
        uniform float fov;
        uniform vec3 sun_color;
        uniform mat4 rotation;
        uniform vec3 eye_position;

        out vec4 frag_color;

        // include cnoise
        {}

        vec3 pixel_direction(vec2 pixel) {{
          // Scale to [0, 1]
          pixel /= window_size;
          // Scale to [-1, 1]
          pixel = 2*pixel - 1;
          // Using vec2(window_size.y) is like using window_size, but then scaling x by aspect ratio.
          pixel *= tan(fov / 2) * vec2(window_size.x / window_size.y, 1);
          return vec3(inverse(rotation) * vec4(pixel, -1, 0));
        }}

        void main() {{
          vec3 direction = pixel_direction(gl_FragCoord.xy);
          float dist = CLOUD_HEIGHT;
          vec3 seed = (eye_position + dist * direction) / CLOUD_HEIGHT * vec3(1, 4, 1);
          float f = cnoise(seed);
          f = sign(f) * pow(abs(f), 15.0/16);
          f = f / 2 + 0.5;
          f = f * f;
          frag_color = vec4(mix(sun_color, vec3(1, 1, 1), f), 1);
        }}"#,
        ::shaders::noise::cnoise(),
      )
    ),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
