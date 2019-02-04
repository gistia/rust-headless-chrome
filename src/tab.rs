use std::cell::RefCell;

use log::*;
use failure::{Error, Fail};

use crate::cdtp::dom;
use crate::cdtp::input;
use crate::cdtp::page::methods::Navigate;
use crate::page_session::PageSession;
use crate::element::Element;
use crate::keys;
use crate::point::Point;

pub type SessionReference = RefCell<PageSession>;

pub struct Tab {
    pub page_session: SessionReference,
}

#[derive(Debug, Fail)]
#[fail(display = "No element found for selector: {}", selector)]
pub struct NoElementFound {
    selector: String
}

impl Tab {
    // TODO: error handling
    pub fn navigate_to(&self, url: &str) -> Result<(), Error> {
        let mut session = self.page_session.borrow_mut();
        let _nav_result = session.call(Navigate { url })?;

        // TODO: at least add a timeout for these loops. it's a disaster waiting to happen!

        trace!("waiting to start navigating");
        // wait for navigating to go to true
        loop {
            if *session.navigating.lock().unwrap() {
                break;
            }
        }
        trace!("started navigating");

        // wait for navigating to go to false
        loop {
            if !*session.navigating.lock().unwrap() {
                break;
            }
        }

        trace!("done navigating");
        Ok(())
    }

    // TODO: have this return a 'can't find element' error when selector returns nothing
    pub fn find_element(&self, selector: &str) -> Result<Element, Error> {
        let node_id = {
            let mut session = self.page_session.borrow_mut();
            // TODO: just do this once.
            let root_node_id = session.call(dom::methods::GetDocument {
                depth: Some(0),
                pierce: Some(false),
            })?.root.node_id;

            session.call(dom::methods::QuerySelector {
                node_id: root_node_id,
                selector,
            })?.node_id
        };

        if node_id == 0 {
            return Err(NoElementFound { selector: selector.to_string() }.into());
        }

        dbg!(node_id);

        let backend_node_id = self.describe_node(node_id)?.backend_node_id;

        dbg!(backend_node_id);

        let remote_object_id = {
            let mut session = self.page_session.borrow_mut();
            let object = session.call(dom::methods::ResolveNode {
                backend_node_id: Some(backend_node_id)
            })?.object;
            object.object_id.expect("couldn't find object ID")
        };
        Ok(Element {
            remote_object_id,
            backend_node_id,
            parent: &self,
        })
    }

    pub fn describe_node(&self, node_id: dom::NodeId) -> Result<dom::Node, Error> {
        let mut session = self.page_session.borrow_mut();
        let node = session.call(dom::methods::DescribeNode {
            node_id: Some(node_id),
            backend_node_id: None,
            depth: Some(100),
        })?.node;
        Ok(node)
    }

    pub fn type_str(&self, string_to_type: &str) -> Result<(), Error> {
        for c in string_to_type.split("") {
            // split call above will have empty string at start and end which we won't type
            if c == "" {
                continue;
            }
            self.press_key(c)?;
        }
        Ok(())
    }

    pub fn press_key(&self, key: &str) -> Result<(), Error> {
        let definition = keys::get_key_definition(key)?;
        let mut session = self.page_session.borrow_mut();

        // TODO: send code and other parts of the def?
        session.call(input::methods::DispatchKeyEvent {
            event_type: "keyDown",
            key: definition.key,
            text: definition.text,
        })?;
        session.call(input::methods::DispatchKeyEvent {
            event_type: "keyUp",
            key: definition.key,
            text: definition.text,
        })?;
        Ok(())
    }


    pub fn click_point(&self, point: Point) -> Result<(), Error> {
        let mut session = self.page_session.borrow_mut();

        session.call(input::methods::DispatchMouseEvent {
            event_type: "mouseMoved",
            x: point.x,
            y: point.y,
        })?;
        session.call(input::methods::DispatchMouseEvent {
            event_type: "mousePressed",
            x: point.x,
            y: point.y,
        })?;
        session.call(input::methods::DispatchMouseEvent {
            event_type: "mouseReleased",
            x: point.x,
            y: point.y,
        })?;
        Ok(())
    }
}