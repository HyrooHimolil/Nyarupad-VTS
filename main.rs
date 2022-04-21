#![windows_subsystem = "windows"]
#![allow(non_snake_case, unused)]
// vim:foldmethod=marker
//{{{
extern crate cgmath;
extern crate ovr_sys;
extern crate raylib;
extern crate vtubestudio;
extern crate once_cell;
extern crate serde;
	
use raylib::prelude::*;
use vtubestudio::{Client, Error};
use vtubestudio::data::ParameterCreationRequest;
use vtubestudio::data::InjectParameterDataRequest;
use vtubestudio::data::ParameterValue;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use once_cell::sync::OnceCell;
use serde::Serialize;
use cgmath::{Matrix, InnerSpace, Array, VectorSpace};
use ovr_sys::*;
use tokio::*;
use vtubestudio::data::StatisticsRequest;

fn connecttry<F>(f: F) -> Result<(), Box<ovrErrorInfo>> where F: FnOnce() -> ovrResult {
    let result = f();
    if OVR_SUCCESS(result) {
        Ok(())
    } else {
        let mut info = Box::new(unsafe{::std::mem::zeroed()});
        unsafe{ ovr_GetLastErrorInfo(&mut *info as *mut _) }
        Err(info)
    }
}

fn updateposition(session: ovrSession) -> ((f32, f32, f32,),(f32, f32, f32)) {
	unsafe{
		let mut trackingstate: ovrTrackingState = ovr_GetTrackingState(session,0.0,0);
		println!("{:?}", trackingstate.HeadPose.ThePose.Orientation);
		let Xyzposition = (trackingstate.HeadPose.ThePose.Position.x, trackingstate.HeadPose.ThePose.Position.y, trackingstate.HeadPose.ThePose.Position.z);
		let mut quat: ovrQuatf = trackingstate.HeadPose.ThePose.Orientation;
		let (radHMDrx,radHMDry,radHMDrz) = cgmath::Quaternion::to_euler(cgmath::Quaternion::new(quat.w, quat.x, quat.y, quat.z));
		let Xyzrotation = (radHMDrx.s,radHMDry.s,radHMDrz.s);
		(Xyzposition,Xyzrotation)
		}
}

fn startOvrConnection() -> (ovrSession) {
	unsafe{
		let mut params: ovrInitParams = ::std::mem::zeroed();
		params.Flags |= ovrInit_RequestVersion;
		params.RequestedMinorVersion = OVR_MINOR_VERSION;
		connecttry(|| ovr_Initialize(&params as *const _)).unwrap();
		let mut session: ovrSession = ::std::mem::zeroed();
		let mut luid: ovrGraphicsLuid = ::std::mem::zeroed();
		connecttry(|| ovr_Create(&mut session as *mut _, &mut luid as *mut _)).unwrap();
		assert!(!session.is_null());
		session
		}
}
//}}}

#[tokio::main]
async fn main() -> Result<(), Error> {
	let C_VER = env!("CARGO_PKG_VERSION");
	let C_NAME = env!("CARGO_PKG_NAME");
	let C_AUTHOR = env!("CARGO_PKG_AUTHORS");
	let funny_cr = "(c) 2022 House Balthazar";
	let connVTS = true;
	let mut smooth = 0.0;
//Connecting{{{
	let ovr_session: ovrSession = startOvrConnection();

    let stored_token = Some("...".to_string());

    let (mut client, mut new_tokens) = Client::builder()
        .auth_token(stored_token)
        .authentication("Starlight Link", "Kate Balthazar", None)
        .build_tungstenite();

    tokio::spawn(async move {
        // This returns whenever the authentication middleware receives a new auth token.
        // We can handle it by saving it somewhere, etc.
        while let Some(token) = new_tokens.next().await {
            println!("Got new auth token: {}", token);
        }
    });

    // Use the client to send a `StatisticsRequest`, handling authentication if necessary.
    // The return type is inferred from the input type to be `StatisticsResponse`.
    let resp = client.send(&StatisticsRequest {}).await?;
    println!("VTube Studio has been running for {}ms", resp.uptime);
//}}}

//Create Parameters{{{
    if connVTS {
        let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_pos_x".to_string(), 
            explanation: Some("hmd_pos_x".to_string()), 
            min: -1.0, 
            max: 1.0, 
            default_value: 0.0
        }).await?;
        let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_pos_y".to_string(), 
            explanation: Some("hmd_pos_y".to_string()), 
            min: -1.0, 
            max: 1.0, 
            default_value: 0.0
        }).await?;
		let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_pos_z".to_string(), 
            explanation: Some("hmd_pos_z".to_string()), 
            min: -1.0, 
            max: 1.0, 
            default_value: 0.0
        }).await?;
        let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_rot_x".to_string(), 
            explanation: Some("hmd_rot_x".to_string()), 
            min: -3.15, 
            max: 3.15, 
            default_value: 0.0
        }).await?;
        let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_rot_y".to_string(), 
            explanation: Some("hmd_rot_y".to_string()), 
            min: -3.15, 
            max: 3.15, 
            default_value: 0.0
        }).await?;
		let resp = client.send(&ParameterCreationRequest {
            parameter_name: "hmd_rot_z".to_string(), 
            explanation: Some("hmd_rot_z".to_string()), 
            min: -3.15, 
            max: 3.15, 
            default_value: 0.0
        }).await?;
    }
//}}}

//Raylib Init{{{
	let width = 400;
	let height = 300;
	let(mut rl, thread) = raylib::init()
		.size(width, height)
		.title(&format!("StarlightLink {}", C_VER))
		.build();
	if !connVTS {rl.set_target_fps(30)}
//}}}

// Load images{{{
	let i_Wicon = Image::load_image("res/icon.png").expect("couldnt load icon image");
	rl.set_window_icon(i_Wicon);
	let i_C = Image::load_image("res/C.png").expect("couldnt load C image");
	let t_C = rl.load_texture_from_image(&thread, &i_C).expect("couldnt load C Texture");
	let i_SL = Image::load_image("res/SL.png").expect("couldnt load SL image");
	let t_SL = rl.load_texture_from_image(&thread, &i_SL).expect("couldnt load SL Texture");
	let i_SR = Image::load_image("res/SR.png").expect("couldnt load SR image");
	let t_SR = rl.load_texture_from_image(&thread, &i_SR).expect("couldnt load SR Texture");
//}}}

	while !rl.window_should_close(){
		smooth = 0.1 / rl.get_frame_time();

// HMD tracking axes{{{
		let ((HMDx,HMDy,HMDz), (HMDrx,HMDry,HMDrz)) = updateposition(ovr_session);
//}}}



// Draw UI/Preview{{{

		let current_fps = rl.get_fps();
		let mut d = rl.begin_drawing(&thread);
		d.clear_background(Color::WHITE);

		d.draw_text(&format!(
"FPS: {}
PARAMETERS
RStickX: {:.2}
RStickY: {:.2}
LStickX: {:.2}
LStickY: {:.2}"
			, current_fps
			, HMDx
			, HMDy
			, HMDrx
			, HMDry
		), 5, 5, 10, Color::BLACK);
	d.draw_texture(&t_C,150,50,Color::WHITE);
	d.draw_texture(&t_SL,150 + (HMDx*5.0) as i32,50 + (HMDy * -1.0 *5.0) as i32,Color::WHITE);
	d.draw_texture(&t_SR,150 + (HMDrx*5.0) as i32,50 + (HMDry * -1.0 *5.0) as i32,Color::WHITE);

	d.draw_text(funny_cr,width - text::measure_text(funny_cr, 10) - 5, height - 10 - 5, 10, Color::BLACK); 
//}}}

// Update Parameters{{{

        if connVTS {
		    client.send(&InjectParameterDataRequest{
		    	parameter_values: vec![ParameterValue{
		    		id: "hmd_pos_x".to_string(),
		    		value: HMDx as f64,
		    		weight: Some(1.0),
					}, ParameterValue{
		    		id: "hmd_pos_y".to_string(),
		    		value: HMDy as f64,
		    		weight: Some(1.0),
					}, ParameterValue{
		    		id: "hmd_pos_z".to_string(),
		    		value: HMDz as f64,
		    		weight: Some(1.0),
					}, ParameterValue{
		    		id: "hmd_rot_x".to_string(),
		    		value: HMDrx as f64,
		    		weight: Some(1.0),
					}, ParameterValue{
		    		id: "hmd_rot_y".to_string(),
		    		value: HMDry as f64,
		    		weight: Some(1.0),
					}, ParameterValue{
		    		id: "hmd_rot_z".to_string(),
		    		value: HMDrz as f64,
		    		weight: Some(1.0),
		    	}],
		    }).await?;
        }
//}}}
	}

    Ok(())
}